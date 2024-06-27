use crate::{
    layout::Layout,
    proof_params::{Fri, ProofParameters, ProverConfig},
};

// https://github.com/cartridge-gg/stone-prover/blob/fd78b4db8d6a037aa467b7558ac8930c10e48dc1/src/starkware/stark/stark.cc#L303-L304
#[cfg(test)]
pub fn fri_degree_bound(proof_params: &ProofParameters) -> u32 {
    let mut expected = proof_params.stark.fri.last_layer_degree_bound;
    for s in &proof_params.stark.fri.fri_step_list {
        expected *= 1 << s
    }
    expected
}

pub fn leaves(proof_params: &ProofParameters) -> Vec<usize> {
    proof_params
        .stark
        .fri
        .fri_step_list
        .iter()
        .skip(1)
        .map(|&x| (1u32 << (x + 4)) - 16)
        .map(|x| x as usize)
        .collect()
}

// https://github.com/cartridge-gg/stone-prover/blob/fd78b4db8d6a037aa467b7558ac8930c10e48dc1/src/starkware/commitment_scheme/packaging_commitment_scheme.cc#L245-L250
pub fn authentications(prover_config: &ProverConfig) -> usize {
    prover_config.constraint_polynomial_task_size as usize + authentication_additional_queries()
}

fn authentication_additional_queries() -> usize {
    // 1
    8
}

pub fn witness(fri: &Fri) -> Vec<usize> {
    let first_fri_step = 16;
    let mut cumulative = 0;
    let mut vec = Vec::new();

    // https://github.com/cartridge-gg/stone-prover/blob/fd78b4db8d6a037aa467b7558ac8930c10e48dc1/src/starkware/fri/fri_details.cc#L93-L97
    for v in fri.fri_step_list.iter().skip(1) {
        cumulative += *v;
        vec.push(first_fri_step - cumulative);
    }

    // https://github.com/cartridge-gg/stone-prover/blob/fd78b4db8d6a037aa467b7558ac8930c10e48dc1/src/starkware/fri/fri_details.cc#L74-L82
    vec.into_iter()
        .map(|len| fri.n_queries * len)
        .map(|x| x as usize)
        .map(|x| x + authentication_additional_queries())
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProofStructure {
    pub first_layer_queries: usize,
    pub layer_count: usize,
    pub composition_decommitment: usize,
    pub oods: usize,
    pub composition_leaves: usize,
    pub last_layer_degree_bound: usize,
    pub authentications: usize,
    pub layer: Vec<usize>,
    pub witness: Vec<usize>,
}

impl ProofStructure {
    pub fn new(
        proof_params: &ProofParameters,
        proof_config: &ProverConfig,
        layout: Layout,
    ) -> Self {
        let n_queries = proof_params.stark.fri.n_queries;
        let mask_len = layout.mask_len();
        let layout = layout.get_consts();

        ProofStructure {
            // https://github.com/cartridge-gg/stone-prover/blob/fd78b4db8d6a037aa467b7558ac8930c10e48dc1/src/starkware/stark/stark.cc#L276-L277
            first_layer_queries: (n_queries * layout.num_columns_first) as usize,

            layer_count: proof_params.stark.fri.fri_step_list.len() - 1,
            composition_decommitment: (n_queries * layout.num_columns_second) as usize,

            // https://github.com/cartridge-gg/stone-prover/blob/fd78b4db8d6a037aa467b7558ac8930c10e48dc1/src/starkware/stark/oods.cc#L92-L93
            oods: mask_len + layout.num_columns_second as usize - 1,
            last_layer_degree_bound: proof_params.stark.fri.last_layer_degree_bound as usize,

            // https://github.com/cartridge-gg/stone-prover/blob/fd78b4db8d6a037aa467b7558ac8930c10e48dc1/src/starkware/stark/composition_oracle.cc#L288-L289
            composition_leaves: 2 * n_queries as usize,
            authentications: authentications(proof_config),

            layer: leaves(proof_params),
            witness: witness(&proof_params.stark.fri),
        }
    }
}

#[test]
fn test_lens() {
    // let n_steps = 16384;
    let layout = Layout::Recursive;
    let proof_params = ProofParameters {
        stark: crate::proof_params::Stark {
            fri: Fri {
                fri_step_list: vec![0, 4, 4, 3],
                last_layer_degree_bound: 128,
                n_queries: 16,
                proof_of_work_bits: 30,
            },
            log_n_cosets: 3,
        },
        n_verifier_friendly_commitment_layers: 0,
    };
    let proof_config = ProverConfig {
        constraint_polynomial_task_size: 256,
        n_out_of_memory_merkle_layers: 1,
        table_prover_n_tasks_per_segment: 1,
    };

    let result = ProofStructure::new(&proof_params, &proof_config, layout);

    let expected = ProofStructure {
        first_layer_queries: 112,
        layer_count: 3,
        composition_decommitment: 48,
        oods: 135,
        last_layer_degree_bound: 128,
        composition_leaves: 32,
        authentications: 256 + 8, // 257
        layer: vec![240, 240, 112],
        witness: vec![193, 129, 81],
    };

    assert_eq!(result, expected);
    assert_eq!(fri_degree_bound(&proof_params), 262144);
}
