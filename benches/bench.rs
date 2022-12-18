use criterion::{criterion_group, criterion_main, BenchmarkGroup, Criterion};
use frodo_pir::api::{CommonParams, QueryParams, Response, Shard};
use pi_rs_cli_utils::*;
use std::time::Duration;

const BENCH_ONLINE: bool = true;
const BENCH_DB_GEN: bool = true;

fn criterion_benchmark(c: &mut Criterion) {
  let CLIFlags {
    m,
    lwe_dim,
    ele_size,
    plaintext_bits,
    ..
  } = parse_from_env();
  let mut lwe_group = c.benchmark_group("lwe");

  println!("Setting up DB for benchmarking. This might take a while...");
  let db_eles = bench_utils::generate_db_eles(m, (ele_size + 7) / 8);
  let shard =
    Shard::from_base64_strings(&db_eles, lwe_dim, m, ele_size, plaintext_bits)
      .unwrap();
  println!("Setup complete, starting benchmarks");
  if BENCH_ONLINE {
    _bench_client_query(&mut lwe_group, &shard);
  }
  if BENCH_DB_GEN {
    lwe_group.sample_size(10);
    lwe_group.measurement_time(Duration::from_secs(100));
    _bench_db_generation(&mut lwe_group, &shard, &db_eles);
  }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

fn _bench_db_generation(
  c: &mut BenchmarkGroup<criterion::measurement::WallTime>,
  shard: &Shard,
  db_eles: &[String],
) {
  let db = shard.get_db();
  let bp = shard.get_base_params();
  let w = db.get_matrix_width_self();

  c.bench_function(
    format!(
      "derive LHS from seed, lwe_dim: {}, m: {}, w: {}",
      bp.get_dim(),
      db.get_matrix_height(),
      w
    ),
    |b| {
      b.iter(|| CommonParams::from(bp));
    },
  );

  println!("Starting DB generation benchmarks");
  c.bench_function(
    format!(
      "generate db and params, m: {}, w: {}",
      db.get_matrix_height(),
      w
    ),
    |b| {
      b.iter(|| {
        Shard::from_base64_strings(
          db_eles,
          bp.get_dim(),
          db.get_matrix_height(),
          db.get_ele_size(),
          db.get_plaintext_bits(),
        )
        .unwrap();
      });
    },
  );
  println!("Finished DB generation benchmarks");
}

fn _bench_client_query(
  c: &mut BenchmarkGroup<criterion::measurement::WallTime>,
  shard: &Shard,
) {
  let db = shard.get_db();
  let bp = shard.get_base_params();
  let cp = CommonParams::from(bp);
  let w = db.get_matrix_width_self();
  let idx = 10;

  println!("Starting client query benchmarks");
  let mut _qp = QueryParams::new(&cp, bp).unwrap();
  let _q = _qp.prepare_query(idx).unwrap();
  let mut _resp = shard.respond(&_q).unwrap();
  c.bench_function(
    format!(
      "create client query params, lwe_dim: {}, m: {}, w: {}",
      bp.get_dim(),
      db.get_matrix_height(),
      w
    ),
    |b| {
      b.iter(|| QueryParams::new(&cp, bp));
    },
  );

  c.bench_function(
    format!(
      "client query prepare, lwe_dim: {}, m: {}, w: {}",
      bp.get_dim(),
      db.get_matrix_height(),
      w
    ),
    |b| {
      b.iter(|| {
        _qp.used = false;
        _qp.prepare_query(idx).unwrap();
      });
    },
  );

  c.bench_function(
    format!(
      "server response compute, lwe_dim: {}, m: {}, w: {}",
      bp.get_dim(),
      db.get_matrix_height(),
      w
    ),
    |b| {
      b.iter(|| {
        shard.respond(&_q).unwrap();
      });
    },
  );

  c.bench_function(
    format!(
      "client parse server response, lwe_dim: {}, m: {}, w: {}",
      bp.get_dim(),
      db.get_matrix_height(),
      w
    ),
    |b| {
      b.iter(|| {
        let deser: Response = bincode::deserialize(&_resp).unwrap();
        deser.parse_output_as_base64(&_qp);
      });
    },
  );
  println!("Finished client query benchmarks");
}

mod bench_utils {
  use rand_core::{OsRng, RngCore};
  pub fn generate_db_eles(num_eles: usize, ele_byte_len: usize) -> Vec<String> {
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
