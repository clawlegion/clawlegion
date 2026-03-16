//! Plugin signature verification for security

use clawlegion_core::{Error, PluginError, Result};
use ring::signature::{self, KeyPair};
use std::path::Path;

/// Signature algorithm
#[derive(Debug, Clone, Copy)]
pub enum SignatureAlgorithm {
    Ed25519,
    RsaPss2048Sha256,
}

/// Plugin signature verifier
pub struct SignatureVerifier {
    public_key: Vec<u8>,
    algorithm: SignatureAlgorithm,
}

impl SignatureVerifier {
    /// Create a new verifier with a public key
    pub fn new(public_key: Vec<u8>, algorithm: SignatureAlgorithm) -> Self {
        Self {
            public_key,
            algorithm,
        }
    }

    /// Create a verifier from a public key file
    pub fn from_key_file(path: &Path, algorithm: SignatureAlgorithm) -> Result<Self> {
        let public_key = std::fs::read(path).map_err(|e| {
            Error::Plugin(PluginError::LoadFailed(format!(
                "Failed to read public key file: {}",
                e
            )))
        })?;

        Ok(Self::new(public_key, algorithm))
    }

    /// Verify a plugin's signature
    pub fn verify(&self, plugin_data: &[u8], signature: &[u8]) -> Result<bool> {
        match self.algorithm {
            SignatureAlgorithm::Ed25519 => {
                let public_key =
                    signature::UnparsedPublicKey::new(&signature::ED25519, &self.public_key);

                match public_key.verify(plugin_data, signature) {
                    Ok(_) => Ok(true),
                    Err(_) => Ok(false),
                }
            }
            SignatureAlgorithm::RsaPss2048Sha256 => {
                let public_key = signature::UnparsedPublicKey::new(
                    &signature::RSA_PSS_2048_8192_SHA256,
                    &self.public_key,
                );

                match public_key.verify(plugin_data, signature) {
                    Ok(_) => Ok(true),
                    Err(_) => Ok(false),
                }
            }
        }
    }

    /// Verify a plugin file
    pub fn verify_plugin_file(&self, plugin_path: &Path) -> Result<bool> {
        // Read plugin binary
        let plugin_data = std::fs::read(plugin_path).map_err(|e| {
            Error::Plugin(PluginError::LoadFailed(format!(
                "Failed to read plugin file: {}",
                e
            )))
        })?;

        // Try to read signature file
        let sig_path = plugin_path.with_extension("sig");
        let signature = std::fs::read(&sig_path).map_err(|e| {
            Error::Plugin(PluginError::LoadFailed(format!(
                "Failed to read signature file: {}",
                e
            )))
        })?;

        self.verify(&plugin_data, &signature)
    }
}

/// Generate a key pair for signing plugins (used during development)
pub fn generate_keypair() -> (Vec<u8>, Vec<u8>) {
    let rng = ring::rand::SystemRandom::new();
    let pkcs8 = signature::Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    let key_pair = signature::Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();

    let public_key = key_pair.public_key().as_ref().to_vec();
    let private_key = pkcs8.as_ref().to_vec();

    (public_key, private_key)
}

/// Sign plugin data
pub fn sign_data(private_key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let key_pair = signature::Ed25519KeyPair::from_pkcs8(private_key).map_err(|e| {
        Error::Plugin(PluginError::LoadFailed(format!(
            "Failed to load private key: {}",
            e
        )))
    })?;

    let signature = key_pair.sign(data);
    Ok(signature.as_ref().to_vec())
}

/// Sign a plugin file
pub fn sign_plugin_file(plugin_path: &Path, private_key: &[u8]) -> Result<Vec<u8>> {
    let plugin_data = std::fs::read(plugin_path).map_err(|e| {
        Error::Plugin(PluginError::LoadFailed(format!(
            "Failed to read plugin file: {}",
            e
        )))
    })?;

    let signature = sign_data(private_key, &plugin_data)?;

    // Write signature to file
    let sig_path = plugin_path.with_extension("sig");
    std::fs::write(&sig_path, &signature).map_err(|e| {
        Error::Plugin(PluginError::LoadFailed(format!(
            "Failed to write signature file: {}",
            e
        )))
    })?;

    Ok(signature)
}
