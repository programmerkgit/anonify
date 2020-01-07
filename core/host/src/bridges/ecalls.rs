use sgx_types::*;
use anonify_types::{RawUnsignedTx, traits::SliceCPtr};
use ed25519_dalek::{Signature, PublicKey};
use crate::auto_ffi::*;
use crate::web3::InnerEnclaveLog;
use crate::error::{HostErrorKind, Result};

pub(crate) fn insert_logs(
    eid: sgx_enclave_id_t,
    enclave_log: &InnerEnclaveLog,
) -> Result<()> {
    let mut rt = sgx_status_t::SGX_ERROR_UNEXPECTED;

    let status = unsafe {
        ecall_insert_logs(
            eid,
            &mut rt,
            enclave_log.contract_addr.as_ptr() as _,
            enclave_log.block_number,
            enclave_log.ciphertexts.as_c_ptr() as *const u8,
            enclave_log.ciphertexts.len() as u32,
            enclave_log.ciphertexts_num,
        )
    };

    if status != sgx_status_t::SGX_SUCCESS {
		return Err(HostErrorKind::Sgx{ status, function: "ecall_insert_logs" }.into());
    }
    if rt != sgx_status_t::SGX_SUCCESS {
		return Err(HostErrorKind::Sgx{ status: rt, function: "ecall_insert_logs" }.into());
    }

    Ok(())
}

/// Get state only if the signature verification returns true.
pub fn get_state(
    eid: sgx_enclave_id_t,
    sig: &Signature,
    pubkey: &PublicKey,
    msg: &[u8],
) -> Result<u64> {
    let mut rt = sgx_status_t::SGX_ERROR_UNEXPECTED;
    let mut res: u64 = Default::default(); // TODO Change type from u64 to Vec<u8>

    let status = unsafe {
        ecall_get_state(
            eid,
            &mut rt,
            sig.to_bytes().as_ptr() as _,
            pubkey.to_bytes().as_ptr() as _,
            msg.as_ptr() as _,
            &mut res,
        )
    };

    if status != sgx_status_t::SGX_SUCCESS {
		return Err(HostErrorKind::Sgx{ status, function: "ecall_get_state" }.into());
    }
    if rt != sgx_status_t::SGX_SUCCESS {
		return Err(HostErrorKind::Sgx{ status: rt, function: "ecall_get_state" }.into());
    }

    Ok(res)
}

/// Initialize a state when a new contract is deployed.
pub fn init_state(
    eid: sgx_enclave_id_t,
    sig: &Signature,
    pubkey: &PublicKey,
    msg: &[u8],
    total_supply: u64,
) -> Result<UnsignedTx> {
    let mut rt = sgx_status_t::SGX_ERROR_UNEXPECTED;
    let mut unsigned_tx = RawUnsignedTx::default();

    let status = unsafe {
        ecall_init_state(
            eid,
            &mut rt,
            sig.to_bytes().as_ptr() as _,
            pubkey.to_bytes().as_ptr() as _,
            msg.as_ptr() as _,
            total_supply,
            &mut unsigned_tx,
        )
    };

    if status != sgx_status_t::SGX_SUCCESS {
		return Err(HostErrorKind::Sgx{ status, function: "ecall_init_state" }.into());
    }
    if rt != sgx_status_t::SGX_SUCCESS {
		return Err(HostErrorKind::Sgx{ status: rt, function: "ecall_init_state" }.into());
    }

    Ok(unsigned_tx.into())
}

/// Update states when a transaction is sent to blockchain.
pub fn state_transition(
    eid: sgx_enclave_id_t,
    sig: &Signature,
    pubkey: &PublicKey,
    msg: &[u8],
    target: &[u8],
    amount: u64,
) -> Result<UnsignedTx> {
    let mut rt = sgx_status_t::SGX_ERROR_UNEXPECTED;
    let mut unsigned_tx = RawUnsignedTx::default();

    let status = unsafe {
        ecall_state_transition(
            eid,
            &mut rt,
            sig.to_bytes().as_ptr() as _,
            pubkey.to_bytes().as_ptr() as _,
            msg.as_ptr() as _,
            target.as_ptr() as _,
            amount,
            &mut unsigned_tx,
        )
    };

    if status != sgx_status_t::SGX_SUCCESS {
		return Err(HostErrorKind::Sgx{ status, function: "ecall_contract_deploy" }.into());
    }
    if rt != sgx_status_t::SGX_SUCCESS {
		return Err(HostErrorKind::Sgx{ status: rt, function: "ecall_contract_deploy" }.into());
    }

    Ok(unsigned_tx.into())
}


#[derive(Debug, Clone, Default)]
pub struct UnsignedTx {
    pub report: Box<[u8]>,
    pub report_sig: Box<[u8]>,
    /// The number of ciphertexts.
    pub ciphertext_num: u32,
    pub ciphertexts: Box<[u8]>,
}

impl From<RawUnsignedTx> for UnsignedTx {
    fn from(raw_tx: RawUnsignedTx) -> Self {
        let mut res_tx: UnsignedTx = Default::default();

        let box_report = raw_tx.report as *mut Box<[u8]>;
        let report = unsafe { Box::from_raw(box_report) };
        let box_report_sig = raw_tx.report_sig as *mut Box<[u8]>;
        let report_sig = unsafe { Box::from_raw(box_report_sig) };
        let box_ciphertexts = raw_tx.ciphertexts as *mut Box<[u8]>;
        let ciphertexts = unsafe { Box::from_raw(box_ciphertexts) };

        res_tx.report = *report;
        res_tx.report_sig = *report_sig;
        res_tx.ciphertexts = *ciphertexts;
        res_tx.ciphertext_num = raw_tx.ciphertext_num;

        res_tx
    }
}


impl UnsignedTx {
    pub fn get_two_ciphertexts(&self) -> (&[u8], &[u8]) {
        self.ciphertexts.split_at(self.ciphertext_num as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::RngCore;
    use rand_os::OsRng;
    use rand::Rng;
    use ed25519_dalek::Keypair;
    use crate::init_enclave::EnclaveDir;

    #[test]
    fn test_init_state() {
        let enclave = EnclaveDir::new().init_enclave(true).unwrap();
        let mut csprng: OsRng = OsRng::new().unwrap();
        let keypair: Keypair = Keypair::generate(&mut csprng);

        let msg = rand::thread_rng().gen::<[u8; 32]>();
        let sig = keypair.sign(&msg);
        assert!(keypair.verify(&msg, &sig).is_ok());

        let total_supply = 100;

        assert!(init_state(
            enclave.geteid(),
            &sig,
            &keypair.public,
            &msg,
            total_supply,
        ).is_ok());
    }

    #[test]
    fn test_state_transition() {
        let enclave = EnclaveDir::new().init_enclave(true).unwrap();
        let mut csprng: OsRng = OsRng::new().unwrap();
        let keypair: Keypair = Keypair::generate(&mut csprng);

        let msg = rand::thread_rng().gen::<[u8; 32]>();
        let sig = keypair.sign(&msg);
        assert!(keypair.verify(&msg, &sig).is_ok());

        let amount = 0;
        let target: [u8; 20] = csprng.gen();

        assert!(state_transition(
            enclave.geteid(),
            &sig,
            &keypair.public,
            &msg,
            &target[..],
            amount,
        ).is_ok());
    }

    #[test]
    #[ignore]
    fn test_ecall_get_state() {
        let enclave = EnclaveDir::new().init_enclave(true).unwrap();
        let mut csprng: OsRng = OsRng::new().unwrap();
        let keypair: Keypair = Keypair::generate(&mut csprng);

        let msg = rand::thread_rng().gen::<[u8; 32]>();
        let sig = keypair.sign(&msg);
        assert!(keypair.verify(&msg, &sig).is_ok());

        let state = get_state(enclave.geteid(), &sig, &keypair.public, &msg);
        // assert_eq!(state, 0);
    }
}