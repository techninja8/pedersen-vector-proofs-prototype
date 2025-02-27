#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_mut)]

use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use rand::rngs::OsRng;
use sha2::{Digest, Sha512};

#[derive(Debug)]
pub struct PedersenVectorCommitment {
    generators: Vec<RistrettoPoint>,
    blinding_generator: RistrettoPoint,
}

#[derive(Debug)]
pub struct Proof {
    index: usize,
    value: Scalar,
    blinding: Scalar,
}

#[derive(Debug)]
pub struct BatchProof {
    indices: Vec<usize>,
    values: Vec<Scalar>,
    blinding: Scalar,
}

impl PedersenVectorCommitment {
    pub fn new(n: usize) -> Self {
        let mut rng = OsRng;
        let generators: Vec<RistrettoPoint> =
            (0..n).map(|_| RistrettoPoint::random(&mut rng)).collect();
        let blinding_generator = RistrettoPoint::random(&mut rng);
        Self {
            generators,
            blinding_generator,
        }
    }

    // We define our cmmitment: C = r * h + ∑_{i=0}^{n-1} x_i * g_i.
    pub fn commit(&self, values: &[Scalar], r: Scalar) -> RistrettoPoint {
        assert_eq!(
            values.len(),
            self.generators.len(),
            "Vector Length Mismatch"
        );
        let mut commitment = r * self.blinding_generator;
        for (x_i, g_i) in values.iter().zip(&self.generators) {
            commitment += g_i * x_i;
        }
        commitment
    }
}

impl Proof {
    // For a single proof, we assume the vector is of length 1.
    fn generate(
        commitment_scheme: &PedersenVectorCommitment,
        values: &[Scalar],
        blinding: Scalar,
        index: usize,
    ) -> Self {
        assert!(index < values.len(), "Index out of bounds!");
        Self {
            index,
            value: values[index],
            blinding,
        }
    }

    // Verification: For a full opening of a 1-element vector (Single-Point Opening),
    // we expect C - x_0 * g_0 == r * h for the proof to hold.
    pub fn verify(
        &self,
        commitment: RistrettoPoint,
        commitment_scheme: &PedersenVectorCommitment,
    ) -> bool {
        let expected_commitment =
            commitment - (commitment_scheme.generators[self.index] * self.value);
        expected_commitment == commitment_scheme.blinding_generator * self.blinding
    }
}

impl BatchProof {
    // For a batch proof, we assume that the entire vector passes is being opened.
    // This is a weak assumption, for the context of this prototype
    pub fn generate(
        commitment_scheme: &PedersenVectorCommitment,
        values: &[Scalar],
        blinding: Scalar,
        indices: &[usize],
    ) -> Self {
        assert!(
            indices.iter().all(|&i| i < values.len()),
            "Index out of bounds!"
        );
        let selected_values: Vec<Scalar> = indices.iter().map(|&i| values[i]).collect();
        Self {
            indices: indices.to_vec(),
            values: selected_values,
            blinding,
        }
    }

    // For simplicity, we use beta = 1 for every opened index.
    // Thus, aggregated_commitment = ∑_{i in indices} x_i * g_i.
    // Verification checks that C - aggregated_commitment == r * h.
    pub fn verify(
        &self,
        commitment: RistrettoPoint,
        commitment_scheme: &PedersenVectorCommitment,
    ) -> bool {
        let betas: Vec<Scalar> = (0..self.indices.len())
            .map(|_| Scalar::from(1u64))
            .collect();
        let aggregated_commitment: RistrettoPoint = self
            .indices
            .iter()
            .zip(self.values.iter())
            .zip(betas.iter())
            .map(|((&idx, &val), &beta)| commitment_scheme.generators[idx] * (beta * val))
            .fold(RistrettoPoint::default(), |a, b| a + b);
        commitment - aggregated_commitment == commitment_scheme.blinding_generator * self.blinding
    }
}

fn main() {}

// Tests
// Tests are simplicied, we do not include considerations for security risks for the sake of this basic prototype
#[cfg(test)]
mod tests {
    use super::*;
    use curve25519_dalek::scalar::Scalar;
    use rand::rngs::OsRng;

    // Test that a commitment and a single proof for a 1-element vector verifies correctly.
    #[test]
    fn test_single_proof_valid() {
        let n = 1;
        let pvp = PedersenVectorCommitment::new(n);
        let values: Vec<Scalar> = vec![Scalar::from(1u64)];
        let mut rng = OsRng;
        let r = Scalar::random(&mut rng);
        let commitment = pvp.commit(&values, r);

        let proof = Proof::generate(&pvp, &values, r, 0);
        assert!(proof.verify(commitment, &pvp), "Proof failed for index 0");
    }

    // Test that a batch proof for a full opening (all indices) verifies correctly.
    #[test]
    fn test_batch_proof_valid() {
        let n = 5;
        let pvp = PedersenVectorCommitment::new(n);
        let values: Vec<Scalar> = vec![
            Scalar::from(1u64),
            Scalar::from(2u64),
            Scalar::from(3u64),
            Scalar::from(4u64),
            Scalar::from(5u64),
        ];
        let mut rng = OsRng;
        let r = Scalar::random(&mut rng);
        let commitment = pvp.commit(&values, r);

        // Open the entire vector.
        let indices: Vec<usize> = (0..n).collect();
        let batch_proof = BatchProof::generate(&pvp, &values, r, &indices);
        assert!(
            batch_proof.verify(commitment, &pvp),
            "Batch proof failed verification"
        );
    }

    // Test that an invalid single proof (tampered value) fails verification.
    #[test]
    fn test_single_proof_invalid() {
        let n = 1;
        let pvp = PedersenVectorCommitment::new(n);
        let values: Vec<Scalar> = vec![Scalar::from(1u64)];
        let mut rng = OsRng;
        let r = Scalar::random(&mut rng);
        let commitment = pvp.commit(&values, r);

        let mut proof = Proof::generate(&pvp, &values, r, 0);
        proof.value = proof.value + Scalar::from(1u64); // Alter the value.
        assert!(
            !proof.verify(commitment, &pvp),
            "Invalid single proof passed verification"
        );
    }

    // Test that an invalid batch proof (tampered one value) fails verification.
    #[test]
    fn test_batch_proof_invalid() {
        let n = 5;
        let pvp = PedersenVectorCommitment::new(n);
        let values: Vec<Scalar> = vec![
            Scalar::from(1u64),
            Scalar::from(2u64),
            Scalar::from(3u64),
            Scalar::from(4u64),
            Scalar::from(5u64),
        ];
        let mut rng = OsRng;
        let r = Scalar::random(&mut rng);
        let commitment = pvp.commit(&values, r);

        let indices: Vec<usize> = (0..n).collect();
        let mut batch_proof = BatchProof::generate(&pvp, &values, r, &indices);
        // Tamper with one of the proof values.
        if let Some(first_value) = batch_proof.values.first_mut() {
            *first_value = *first_value + Scalar::from(1u64);
        }
        assert!(
            !batch_proof.verify(commitment, &pvp),
            "Invalid batch proof passed verification"
        );
    }
}