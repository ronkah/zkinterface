use rand;
use num_bigint::{BigUint, RandBigInt};
use num_traits::{Zero, One, Num};

use crate::{Result, CircuitHeader, ConstraintSystem, Variables, Witness, WorkspaceSink, StatementBuilder, Sink};
use std::path::Path;
use rand::Rng;

/// Will generate a constraint system as a system of quadratic equations.
/// SUM_i ( SUM_j ( lambda_{i,j} * w_i * w_j ) )  =  c_k
///
/// The witnesses are x_i variables, while the instances variables are the c_i ones.
/// This can be expressed easily as a R1CS system using the following trick:
///   - Generate first a bunch of witnesses of size 'wit_nbr' in the underlying field,
///   - Generate 'ins_nbr' pairs of binary vectors, each of size wit_nbr (named b_1, b_2)
///   - Compute the product   (witness * b_1) * (witness * b_2) = c_i
///   - The result is then stored in c_i, a newly created instance variable

/// This is the list of all primes characteristics to be used for generating constraints.
/// They are written as hexadecimal strings, big endian.
const BENCHMARK_PRIMES: [&str; 4]  = [
    "2",   // 2
    "11",   // 17
    "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFF61", // 128-bits prime: 2**128 - 158
    "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF43", // 256 prime: 2**256 - 189
];

/// This is the list of sizes for the list of witnesses.
const BENCHMARK_CS_WITNESS_NUMBER: [u64; 4] = [
    3,
    100,
    1000,
    1000000
];

/// This is the list of sizes for the list of instance variables.
const BENCHMARK_CS_INSTANCES_NUMBER: [u64; 3]  = [
    10,
    10000,
    1000000
];


struct BenchmarkParameter {
    pub ins_number: u64,
    pub wit_number: u64,
    pub modulus: BigUint,
    pub workspace: WorkspaceSink,
}

impl BenchmarkParameter {
    pub fn new(workspace_: impl AsRef<Path>, ins_number_: u64, wit_number_: u64, hexaprime: &str) -> Result<BenchmarkParameter> {
        let ws = workspace_.as_ref().to_path_buf();
        let ws = WorkspaceSink::new(ws.join(format!("metrics_{}_{}_{}/", hexaprime, ins_number_, wit_number_)))?;
        let prime = BigUint::from_str_radix(hexaprime, 16)?;
        Ok(BenchmarkParameter {
            ins_number: ins_number_,
            wit_number: wit_number_,
            modulus: prime,
            workspace: ws,
        })
    }
}

impl Sink for BenchmarkParameter {
    fn push_header(&mut self, header: CircuitHeader) -> Result<()> { self.workspace.push_header(header) }
    fn push_constraints(&mut self, cs: ConstraintSystem) -> Result<()> { self.workspace.push_constraints(cs) }
    fn push_witness(&mut self, witness: Witness) -> Result<()> { self.workspace.push_witness(witness) }
}

macro_rules! bits_to_bytes {
    ($val:expr) => (($val + 7) / 8);
}

/// This function will generate R1CS constraints systems for all parameters
/// set here.
///
/// # Arguments
///
/// * `workspace_`  - a PathBuf giving the output directory
///
pub fn generate_all_metrics_data(workspace_: impl AsRef<Path>) -> Result<()> {

    for hexaprime in BENCHMARK_PRIMES.iter() {
        for wit_nbr in BENCHMARK_CS_WITNESS_NUMBER.iter() {
            for ins_nbr in BENCHMARK_CS_INSTANCES_NUMBER.iter() {
                println!("Generating R1CS system for prime:{} / witness number: {} / instance number: {}", &hexaprime, *wit_nbr, *ins_nbr);
                generate_metrics_data(&workspace_, &hexaprime, *wit_nbr, *ins_nbr)?;
            }
        }
    }

    Ok(())
}

/// This function will generate a R1CS constraints system, and stores it in the directory specified
/// as parameter, for the given field characteristic, with the given number of witnesses / instances
/// variables. It returns an `Error<>`.
///
/// # Arguments
///
/// * `workspace_`  - a PathBuf giving the output directory
/// * `hexaprime`  - a string slice containing the hexadecimal representation of the prime
/// * `wit_nbr` - a integer giving the number of witnesses the generated system should have
/// * `ins_nbr` - a integer giving the number of instance variables the generated system should have
///
/// # Examples
/// To generate a R1CS system with 10 witnesses, 20 instance variables, over the field F_{257}
/// ```
///     use std::fs::remove_dir_all;
///     use std::path::PathBuf;
///     use zkinterface::producers::circuit_generator::generate_metrics_data;
///
///     let workspace = PathBuf::from("local/test_metrics");
///     let _ = remove_dir_all(&workspace);
///     let prime: &str = "101";  // 257 which is prime
///
///     match generate_metrics_data(&workspace, &prime, 10, 20) {
///         Err(_) => eprintln!("Error"),
///         _ => {}
///     }
/// ```
///
pub fn generate_metrics_data(workspace_: impl AsRef<Path>, hexaprime: &str, wit_nbr: u64, ins_nbr: u64) -> Result<()> {
    let mut rng = rand::thread_rng();
    let empty_constraints: Vec<((Vec<u64>, Vec<u8>), (Vec<u64>, Vec<u8>), (Vec<u64>, Vec<u8>))> = vec![];

    let bp = BenchmarkParameter::new(&workspace_, ins_nbr, wit_nbr, &hexaprime)?;
    let size_in_bytes = bits_to_bytes!(&bp.modulus.bits()) as usize;
    let witnesses: Vec<BigUint> = (0..wit_nbr).map(|_| rng.gen_biguint_below(&bp.modulus)).collect();
    let mut builder = StatementBuilder::new(bp);

    builder.header.field_maximum = Some(serialize_biguint(&builder.sink.modulus - BigUint::one(), size_in_bytes));
    let wit_idx = ((ins_nbr+1)..(ins_nbr + wit_nbr + 1)).collect::<Vec<u64>>();

    let mut constraints_vec: Vec<((Vec<u64>, Vec<u8>), (Vec<u64>, Vec<u8>), (Vec<u64>, Vec<u8>))> = vec![
        // (A ids values)  *  (B ids values)  =  (C ids values)
        ((vec![0], vec![1]), (wit_idx.clone(), vec![1; wit_idx.len()]), (wit_idx.clone(), vec![1; wit_idx.len()])),
    ];

    for _i in 0..ins_nbr {
        let b1: Vec<u8> = (0..wit_nbr).map(|_| rng.gen_range(0, 2)).collect();
        let b2: Vec<u8> = (0..wit_nbr).map(|_| rng.gen_range(0, 2)).collect();

        let mut result_of_equation = compute_equation(&witnesses, &b1) * compute_equation(&witnesses, &b2);
        result_of_equation %= &builder.sink.modulus;
        let buf = serialize_biguint(result_of_equation, size_in_bytes);

        let instance_id = builder.allocate_instance_var(&buf);

        let mut new_constraint: Vec<((Vec<u64>, Vec<u8>), (Vec<u64>, Vec<u8>), (Vec<u64>, Vec<u8>))> = vec![
            ((wit_idx.clone(), b1), (wit_idx.clone(), b2), (vec![instance_id], vec![1]))
        ];
        constraints_vec.append(&mut new_constraint);
        if constraints_vec.len() >= 100000 {
            builder.push_constraints(ConstraintSystem::from(constraints_vec.as_ref()))?;
            constraints_vec = empty_constraints.clone();
        }

    }

    // if it remains constraints not yet pushed, push them.
    if constraints_vec.len() > 0 {
        builder.push_constraints(ConstraintSystem::from(constraints_vec.as_ref()))?;
    }

    let witness_buffer = serialize_biguints(witnesses, size_in_bytes);

    builder.push_witness(Witness {
        assigned_variables: Variables {
            variable_ids: wit_idx, // xx, yy
            values: Some(witness_buffer),
        }
    })?;
    builder.header.free_variable_id += wit_nbr + 1;
    builder.finish_header()?;
    Ok(())
}

/// serialize `BigUint` as a vector of u8 (in Little-endian), of a given size, padded with 0's if required.
fn serialize_biguint(biguint: BigUint, byte_len: usize) -> Vec<u8> {
    let mut ret: Vec<u8> = Vec::new();
    let mut binary = biguint.to_bytes_le();
    let len = binary.len();
    ret.append(&mut binary);
    ret.append(&mut vec![0; byte_len - len]);

    ret
}

/// serialize a `Vec<BigUint>` as a vector of u8, each `BigUint` exported a fixed-size vector of u8
/// padded with 0's if required.
fn serialize_biguints(biguints: Vec<BigUint>, byte_len: usize) -> Vec<u8> {
    let mut ret: Vec<u8> = Vec::new();

    for biguint in biguints.iter() {
        ret.append(&mut serialize_biguint(biguint.clone(), byte_len));
    }
    ret
}

fn compute_equation(witnesses: &Vec<BigUint>, bit_vector: &[u8]) -> BigUint {
    let mut ret = BigUint::zero();
    let _: Vec<usize> = witnesses.iter().enumerate()
                .filter(|(idx, _buint)| bit_vector[*idx] != 0 )
                .map(|(idx, buint)| {
                    ret += buint;
                    idx
                }).collect();

    ret
}



#[test]
fn test_generate_metrics() {
    use std::fs::remove_dir_all;
    use std::path::PathBuf;
    use crate::consumers::simulator::Simulator;
    use crate::consumers::workspace::Workspace;

    let workspace = PathBuf::from("local/test_metrics");
    let _ = remove_dir_all(&workspace);

    match generate_metrics_data(&workspace, &BENCHMARK_PRIMES[0], 500, 500) {
        Err(_) => eprintln!("Error"),
        _ => {}
    }

    // Check whether the statement is true.
    let mut simulator = Simulator::default();
    let ws: Workspace = Workspace::from_dir(&workspace.join(format!("metrics_{}_500_500/", &BENCHMARK_PRIMES[0])) ).unwrap();

    // Must validate and simulate in parallel to support stdin.
    for msg in ws.iter_messages() {
        simulator.ingest_message(&msg);
    }

    assert_eq!(simulator.get_violations().len(), 0);

}