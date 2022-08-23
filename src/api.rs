use crate::db::Database;
pub use crate::db::{BaseParams, CommonParams};
pub use crate::utils::format::*;
use crate::utils::lwe::*;
use crate::utils::matrices::*;
use errors::PIRError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::str;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shard {
    db: Database,
    base_params: BaseParams,
}
impl Shard {
    /// Expects a JSON file of base64-encoded strings in file path. It also
    /// expects the lwe dimension, m (the number of DB elements), element size
    /// (in bytes) of the database elements, and plaintext bits.
    /// It will call the 'from_base64_strings' function to generate the database.
    pub fn from_json_file(
        file_path: &str,
        lwe_dim: usize,
        m: usize,
        ele_size: usize,
        plaintext_bits: usize,
    ) -> Self {
        let file_contents: String = fs::read_to_string(file_path).unwrap().parse().unwrap();
        let elements: Vec<String> = serde_json::from_str(&file_contents).unwrap();
        Shard::from_base64_strings(&elements, lwe_dim, m, ele_size, plaintext_bits)
    }

    /// Expects an array of base64-encoded strings and converts into a
    /// database that can process client queries
    pub fn from_base64_strings(
        base64_strs: &[String],
        lwe_dim: usize,
        m: usize,
        ele_size: usize,
        plaintext_bits: usize,
    ) -> Self {
        let db = Database::new(base64_strs, m, ele_size, plaintext_bits);
        let base_params = BaseParams::new(&db, lwe_dim);
        Self { db, base_params }
    }

    /// Write base_params and DB to file
    pub fn write_to_file(&self, db_path: &str, params_path: &str) {
        self.db.write_to_file(db_path);
        self.base_params.write_to_file(params_path);
    }

    /// Produces a serialized response (base64-encoded) to a serialized
    /// client query
    pub fn respond(&self, q: &Query) -> Result<Vec<u8>, PIRError> {
        let resp = Response(
            (0..self.db.get_matrix_width_self())
                .into_iter()
                .map(|i| self.db.vec_mult(q.as_slice(), i))
                .collect(),
        );
        let se = bincode::serialize(&resp);
        if let Err(e) = se {
            return Err(PIRError::SerdeError(e.to_string()));
        }
        Ok(se.unwrap())
    }

    /// Returns the database
    pub fn get_db(&self) -> &Database {
        &self.db
    }

    /// Returns the base parameters
    pub fn get_base_params(&self) -> &BaseParams {
        &self.base_params
    }

    // TODO: alex: do we need this?
    pub fn into_row_iter(&self) -> std::vec::IntoIter<std::string::String> {
        (0..self.get_db().get_matrix_height())
            .into_iter()
            .map(|i| self.get_db().get_db_entry(i))
            .collect::<Vec<String>>()
            .into_iter()
    }
}

/// The `QueryParams` is a structure to be used for a client
/// query.
#[derive(Clone, Debug)]
pub struct QueryParams {
    lhs: Vec<u32>,
    rhs: Vec<u32>,
    ele_size: usize,
    plaintext_bits: usize,
    pub used: bool,
}
impl QueryParams {
    pub fn new(cp: &CommonParams, bp: &BaseParams) -> Self {
        let s = random_ternary_vector(bp.get_dim());
        Self {
            lhs: cp.mult_left(&s),
            rhs: bp.mult_right(&s),
            ele_size: bp.get_ele_size(),
            plaintext_bits: bp.get_plaintext_bits(),
            used: false,
        }
    }

    /// Prepares a new client query based on row_index (given as input)
    pub fn prepare_query(&mut self, row_index: usize) -> Query {
        self.used = true;
        let query_indicator = get_rounding_factor(self.plaintext_bits);
        let mut lhs = self.lhs.clone();
        lhs[row_index] += query_indicator;
        Query(lhs)
    }
}

/// The `Query` struct holds the necessary information encoded in
/// a client PIR query to the server DB for a particular `row_index`. It
/// provides methods for parsing server responses.
#[derive(Clone, Serialize, Deserialize)]
pub struct Query(Vec<u32>);
impl Query {
    pub fn as_slice(&self) -> &[u32] {
        &self.0
    }
}

/// The `Response` object wraps a response from a single shard
#[derive(Clone, Serialize, Deserialize)]
pub struct Response(Vec<u32>);
impl Response {
    pub fn as_slice(&self) -> &[u32] {
        &self.0
    }

    /// Parses the output as a row of u32 values
    pub fn parse_output_as_row(&self, qp: &QueryParams) -> Vec<u32> {
        // get parameters for rounding
        let rounding_factor = get_rounding_factor(qp.plaintext_bits);
        let rounding_floor = get_rounding_floor(qp.plaintext_bits);
        let plaintext_size = get_plaintext_size(qp.plaintext_bits);

        // perform division and rounding
        (0..Database::get_matrix_width(qp.ele_size, qp.plaintext_bits))
            .into_iter()
            .map(|i| {
                let unscaled_res = self.0[i].wrapping_sub(qp.rhs[i]);
                let scaled_res = unscaled_res / rounding_factor;
                let scaled_rem = unscaled_res % rounding_factor;
                let mut rounded_res = scaled_res;
                if scaled_rem > rounding_floor {
                    rounded_res += 1;
                }
                rounded_res % plaintext_size
            })
            .collect()
    }

    /// Parses the output as a base64-encoded string
    pub fn parse_output_as_base64(&self, qp: &QueryParams) -> String {
        let row = self.parse_output_as_row(qp);
        base64_from_u32_slice(&row, qp.plaintext_bits, qp.ele_size)
    }
}

pub mod errors {
    use std::fmt;

    #[derive(Debug, Clone, PartialEq)]
    pub enum PIRError {
        UsedQueryParamsError,
        SerdeError(String),
    }

    impl std::error::Error for PIRError {}

    impl fmt::Display for PIRError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                PIRError::UsedQueryParamsError => write!(
                    f,
                    "Query parameters have already been used for generating client query."
                ),
                PIRError::SerdeError(e) => write!(f, "Error occurred during serialization: {}.", e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::{OsRng, RngCore};

    #[test]
    fn client_query_e2e() {
        let lwe_dim = 512;
        let m = 2u32.pow(12) as usize;
        let ele_size = 2u32.pow(8) as usize;
        let plaintext_bits = 12usize;
        let db_eles = generate_db_eles(m, (ele_size + 7) / 8);
        let shard = Shard::from_base64_strings(&db_eles, lwe_dim, m, ele_size, plaintext_bits);
        let base_params = shard.get_base_params();
        let common_params = CommonParams::from(base_params);
        #[allow(clippy::needless_range_loop)]
        for i in 0..10 {
            let mut query_params = QueryParams::new(&common_params, base_params);
            let query = query_params.prepare_query(i);
            let d_resp = shard.respond(&query).unwrap();
            let resp: Response = bincode::deserialize(&d_resp).unwrap();
            let output = resp.parse_output_as_base64(&query_params);
            assert_eq!(output, db_eles[i]);
        }
    }

    #[test]
    fn attempt_params_reuse() {
        let m = 2u32.pow(6) as usize;
        let ele_size = 2u32.pow(8) as usize;
        let plaintext_bits = 10usize;
        let dim = 512;
        let db_eles = generate_db_eles(m, (ele_size + 7) / 8);
        let shard = Shard::from_base64_strings(&db_eles, dim, m, ele_size, plaintext_bits);
        let bp = shard.get_base_params();
        let cp = CommonParams::from(bp);
        let mut qp = QueryParams::new(&cp, bp);
        // should be successful in generating a query
        qp.prepare_query(0);

        // should assert it is deemed "used"
        assert!(qp.used);
    }

    // Helper function used for testing. Generates random database elements
    fn generate_db_eles(num_eles: usize, ele_byte_len: usize) -> Vec<String> {
        let mut eles = Vec::with_capacity(num_eles);
        for _ in 0..num_eles {
            let mut ele = vec![0u8; ele_byte_len];
            OsRng.fill_bytes(&mut ele);
            let ele_str = base64::encode(ele);
            eles.push(ele_str);
        }
        eles
    }
}
