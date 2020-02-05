use crate::localstd::{
    io::{self, Read, Write},
    vec::Vec,
};
use crate::{
    stf::STATE_SIZE,
    serde::{Serialize, Deserialize}
};
use ed25519_dalek::{PublicKey, Signature, Keypair, SignatureError};
use tiny_keccak::Keccak;
use anonify_types::RawAccessRight;
#[cfg(feature = "std")]
use rand::Rng;
#[cfg(feature = "std")]
use rand_core::{RngCore, CryptoRng};

/// Trait for 256-bits hash functions
pub trait Hash256 {
    fn hash(inp: &[u8]) -> Self;

    fn from_pubkey(pubkey: &PublicKey) -> Self;
}

/// User address represents last 20 bytes of digest of user's public key.
/// A signature verification must return true to generate a user address.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(crate = "crate::serde")]
pub struct UserAddress([u8; 20]);

#[cfg(feature = "std")]
impl From<UserAddress> for web3::types::Address {
    fn from(address: UserAddress) -> Self {
        let bytes = address.as_bytes();
        web3::types::Address::from_slice(bytes)
    }
}

#[cfg(feature = "std")]
impl From<&UserAddress> for web3::types::Address {
    fn from(address: &UserAddress) -> Self {
        let bytes = address.as_bytes();
        web3::types::Address::from_slice(bytes)
    }
}

impl UserAddress {
    /// Get a user address only if the verification of signature returns true.
    pub fn from_sig(msg: &[u8], sig: &Signature, pubkey: &PublicKey) -> Result<Self, SignatureError> {
        pubkey.verify(msg, &sig)?;
        Ok(Self::from_pubkey(&pubkey))
    }

    pub fn from_access_right(access_right: &AccessRight) -> Result<Self, SignatureError> {
        access_right.verify_sig()?;
        Ok(Self::from_pubkey(access_right.pubkey()))
    }

    pub fn from_pubkey(pubkey: &PublicKey) -> Self {
        let hash = Sha256::from_pubkey(pubkey);
        let addr = &hash.as_array()[12..];
        let mut res = [0u8; 20];
        res.copy_from_slice(addr);

        UserAddress(res)
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.0)?;
        Ok(())
    }

    pub fn read<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut res = [0u8; 20];
        reader.read_exact(&mut res)?;
        Ok(UserAddress(res))
    }

    #[cfg(feature = "std")]
    pub fn base64_encode(&self) -> String {
        base64::encode(self.as_bytes())
    }

    #[cfg(feature = "std")]
    pub fn base64_decode(encoded_str: &str) -> Self {
        let decoded_vec = base64::decode(encoded_str).expect("Faild to decode base64.");
        assert_eq!(decoded_vec.len(), 20);

        let mut arr = [0u8; 20];
        arr.copy_from_slice(&decoded_vec[..]);

        UserAddress::from_array(arr)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0[..]
    }

    pub fn from_array(array: [u8; 20]) -> Self {
        UserAddress(array)
    }
}

/// Hash digest of sha256 hash function
#[derive(Clone, Default, Debug, PartialEq)]
pub struct Sha256([u8; 32]);

impl Hash256 for Sha256 {
    fn hash(inp: &[u8]) -> Self {
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.input(inp);

        let mut res = Sha256::default();
        res.copy_from_slice(&hasher.result());
        res
    }

    fn from_pubkey(pubkey: &PublicKey) -> Self {
        Self::hash(&pubkey.to_bytes())
    }
}

impl Sha256 {
    pub fn as_array(&self) -> [u8; 32] {
        self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0[..]
    }

    fn copy_from_slice(&mut self, src: &[u8]) {
        self.0.copy_from_slice(src)
    }
}

/// A trait that will hash using Keccak256 the object it's implemented on.
pub trait Keccak256<T> {
    /// This will return a sized object with the hash
    fn keccak256(&self) -> T where T: Sized;
}

impl Keccak256<[u8; 32]> for [u8] {
    fn keccak256(&self) -> [u8; 32] {
        let mut keccak = Keccak::new_keccak256();
        let mut result = [0u8; 32];
        keccak.update(self);
        keccak.finalize(result.as_mut());
        result
    }
}

/// Access right of Read/Write to anonify's enclave mem db.
#[derive(Debug, Clone)]
pub struct AccessRight {
    pub sig: Signature,
    pub pubkey: PublicKey,
    pub challenge: [u8; 32],
}

impl AccessRight {
    #[cfg(feature = "std")]
    pub fn new_from_rng<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let keypair = Keypair::generate(rng);
        let challenge = rand::thread_rng().gen::<[u8; 32]>();
        let sig = keypair.sign(&challenge);

        assert!(keypair.verify(&challenge, &sig).is_ok());

        Self::new(sig, keypair.public, challenge)
    }

    pub fn new(
        sig: Signature,
        pubkey: PublicKey,
        challenge: [u8; 32],
    ) -> Self {
        assert!(pubkey.verify(&challenge, &sig).is_ok());

        AccessRight {
            sig,
            pubkey,
            challenge,
        }
    }

    pub fn verify_sig(&self) -> Result<(), SignatureError> {
        self.pubkey.verify(&self.challenge, &self.sig)?;
        Ok(())
    }

    pub fn user_address(&self) -> UserAddress {
        UserAddress::from_pubkey(&self.pubkey())
    }

    pub fn pubkey(&self) -> &PublicKey {
        &self.pubkey
    }

    pub fn into_raw(self) -> RawAccessRight {
        RawAccessRight {
            sig: self.sig.to_bytes().as_ptr(),
            pubkey: self.pubkey().to_bytes().as_ptr(),
            challenge: self.challenge.as_ptr(),
        }
    }

    pub fn from_raw(raw_ar: RawAccessRight) -> Self {
        unimplemented!();
    }
}

/// The size of initialization vector for AES-256-GCM.
pub const IV_SIZE: usize = 12;
pub const CIPHERTEXT_SIZE: usize = STATE_SIZE + IV_SIZE;

#[derive(Debug, Clone)]
pub struct Ciphertext([u8; CIPHERTEXT_SIZE]);

impl Ciphertext {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), CIPHERTEXT_SIZE);
        let mut buf = [0u8; CIPHERTEXT_SIZE];
        buf.copy_from_slice(bytes);

        Ciphertext(buf)
    }

    pub fn from_bytes_iter(bytes: &[u8]) -> impl Iterator<Item=Self> + '_ {
        assert_eq!(bytes.len() % CIPHERTEXT_SIZE, 0);
        let iter_num = bytes.len() / CIPHERTEXT_SIZE;

        (0..iter_num).map(move |i| {
            let mut buf = [0u8; CIPHERTEXT_SIZE];
            let b = &bytes[i*CIPHERTEXT_SIZE..(i+1)*CIPHERTEXT_SIZE];
            buf.copy_from_slice(b);
            Ciphertext(buf)
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0[..]
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

const LOCK_PARAM_SIZE: usize = 32;

/// To avoid data collision when a transaction is sent to a blockchain.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LockParam([u8; LOCK_PARAM_SIZE]);

impl LockParam {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), LOCK_PARAM_SIZE);
        let mut buf = [0u8; LOCK_PARAM_SIZE];
        buf.copy_from_slice(bytes);

        LockParam(buf)
    }

    pub fn from_bytes_iter(bytes: &[u8]) -> impl Iterator<Item=Self> + '_ {
        assert_eq!(bytes.len() % LOCK_PARAM_SIZE, 0);
        let iter_num = bytes.len() / LOCK_PARAM_SIZE;

        (0..iter_num).map(move |i| {
            let mut buf = [0u8; LOCK_PARAM_SIZE];
            let b = &bytes[i*LOCK_PARAM_SIZE..(i+1)*LOCK_PARAM_SIZE];
            buf.copy_from_slice(&b);
            LockParam(buf)
        })
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0[..]
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.0)?;
        Ok(())
    }

    pub fn read<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut res = [0u8; 32];
        reader.read_exact(&mut res)?;
        Ok(LockParam(res))
    }
}

impl From<Sha256> for LockParam {
    fn from(s: Sha256) -> Self {
        LockParam(s.as_array())
    }
}
