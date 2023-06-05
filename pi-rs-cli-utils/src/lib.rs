use clap::{Parser, arg};
use std::env;
use std::num::ParseIntError;

#[derive(Parser)]
/// Flags for setting PIR parameters
pub struct CLIFlags {
  /// Log2 of height of DB matrix
  #[arg(short, long, default_value_t = 16, value_parser = parse_exp_to_usize)]
  pub matrix_height: usize,
  /// LWE dimension
  #[arg(short = 'd', long = "dim", default_value_t = 2048)]
  pub lwe_dim: usize,
  /// Number of plaintext bits encoded in each entry of DB matrix
  #[arg(short, long, default_value_t = 10)]
  pub plaintext_bits: usize,
  /// Log2 of element bit length
  #[arg(short, long, default_value_t = 13, value_parser = parse_exp_to_usize)]
  pub ele_size: usize,
}

pub fn parse_cli_flags() -> CLIFlags {
    CLIFlags::parse()
}

pub fn parse_from_env() -> CLIFlags {
  let ele_size = parse_exp_to_usize(&env::var("PIR_ELE_SIZE_EXP").unwrap()).unwrap();
  let lwe_dim: usize = env::var("PIR_LWE_DIM").unwrap().parse().unwrap();
  let plaintext_bits: usize =
    env::var("PIR_PLAINTEXT_BITS").unwrap().parse().unwrap();
  let matrix_height = parse_exp_to_usize(&env::var("PIR_MATRIX_HEIGHT_EXP").unwrap()).unwrap();
  CLIFlags {
    matrix_height,
    lwe_dim,
    plaintext_bits,
    ele_size,
  }
}

fn parse_exp_to_usize(v: &str) -> Result<usize, ParseIntError> {
  let exp: u32 = v.parse()?;
  Ok(2_u32.pow(exp) as usize)
}
