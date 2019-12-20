use sgx_types::sgx_enclave_id_t;
use log::debug;
use anonify_common::UserAddress;
use ed25519_dalek::{Signature, PublicKey};
use ::web3::types::H256;
use crate::{
    init_enclave::EnclaveDir,
    ecalls::*,
    error::Result,
    web3,
};

pub fn init_enclave() -> sgx_enclave_id_t {
    #[cfg(not(debug_assertions))]
    let enclave = EnclaveDir::new().init_enclave(false).unwrap();
    #[cfg(debug_assertions)]
    let enclave = EnclaveDir::new().init_enclave(true).unwrap();

    enclave.geteid()
}

pub fn anonify_deploy(
    enclave_id: sgx_enclave_id_t,
    sig: &Signature,
    pubkey: &PublicKey,
    nonce: &[u8],
    total_supply: u64,
    eth_url: &str,
) -> Result<[u8; 20]> {
    let unsigned_tx = init_state(
        enclave_id,
        sig,
        pubkey,
        nonce,
        total_supply,
    )?;

    debug!("unsigned_tx: {:?}", &unsigned_tx);

    let web3_conn = web3::Web3Http::new(eth_url)?;
    let deployer = web3_conn.get_account(0)?;
    let contract_addr = web3_conn.deploy(
        deployer,
        &unsigned_tx.ciphertexts,
        &unsigned_tx.report,
        &unsigned_tx.report_sig,
    )?;

    Ok(contract_addr.to_fixed_bytes())
}

pub fn anonify_send(
    enclave_id: sgx_enclave_id_t,
    sig: &Signature,
    pubkey: &PublicKey,
    nonce: &[u8],
    target: &UserAddress,
    amount: u64,
    contract: &web3::AnonymousAssetContract,
    gas: u64,
) -> Result<H256> {
    let from_addr = UserAddress::from_pubkey(&pubkey);

    let unsigned_tx = state_transition(
        enclave_id,
        sig,
        pubkey,
        nonce,
        target.as_bytes(),
        amount,
    )?;

    debug!("unsigned_tx: {:?}", &unsigned_tx);

    let (update_bal1, update_bal2) = unsigned_tx.get_two_ciphertexts();
    let receipt = contract.tranfer::<u64>(
        from_addr.into(),
        update_bal1,
        update_bal2,
        &unsigned_tx.report,
        &unsigned_tx.report_sig,
        gas,
    )?;

    Ok(receipt)
}

pub fn anonify_get_state(
    enclave_id: sgx_enclave_id_t,
    sig: &Signature,
    pubkey: &PublicKey,
    nonce: &[u8],
) -> Result<u64> {
    let state = get_state(
        enclave_id,
        sig,
        pubkey,
        nonce,
    )?;

    debug!("state: {:?}", &state);
    Ok(state)
}
