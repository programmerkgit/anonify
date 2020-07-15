use std::{
    slice,
    marker::PhantomData,
};
use sgx_types::*;
use anonify_types::*;
use anonify_common::{
    crypto::{UserAddress, AccessRight, Ciphertext},
    traits::*,
    state_types::{MemId, StateType},
    plugin_types::*,
};
use anonify_runtime::traits::*;
use anonify_treekem::handshake::HandshakeParams;
use ed25519_dalek::{PublicKey, Signature};
use codec::{Decode, Encode};
use log::debug;
use anyhow::{Result, anyhow};
use crate::{
    transaction::*,
    instructions::Instructions,
    bridges::ocalls::save_to_host_memory,
    context::EnclaveContext,
};

pub trait EcallHandler {
    type O: EcallOutput + Encode;

    fn handle<R, C>(
        self,
        enclave_context: &C,
        max_mem_size: usize,
    ) -> Result<Self::O>
    where
        R: RuntimeExecutor<C, S=StateType>,
        C: ContextOps<S=StateType> + Clone;
}

impl EcallHandler for input::Instruction {
    type O = output::Instruction;

    fn handle<R, C>(
        mut self,
        enclave_context: &C,
        max_mem_size: usize
    ) -> Result<Self::O>
    where
        R: RuntimeExecutor<C, S=StateType>,
        C: ContextOps<S=StateType> + Clone,
    {
        let state = self.state.as_mut_bytes();
        let ar = &self.access_right;

        let instruction_tx = construct_instruction::<R, C>(
            self.call_id,
            state,
            ar,
            enclave_context,
            max_mem_size,
        )?;

        let addr = ar.verified_user_address()?;
        enclave_context.set_notification(addr);
        Ok(instruction_tx)
    }
}

impl EcallHandler for input::InsertCiphertext {
    type O = output::ReturnUpdatedState;

    fn handle<R, C>(
        self,
        enclave_context: &C,
        _max_mem_size: usize
    ) -> Result<Self::O>
    where
        R: RuntimeExecutor<C, S=StateType>,
        C: ContextOps<S=StateType> + Clone,
    {
        let group_key = &mut *enclave_context.write_group_key();
        let iter_op = Instructions::<R, C>::state_transition(enclave_context.clone(), self.ciphertext(), group_key)?;
        let mut output = output::ReturnUpdatedState::default();

        if let Some(updated_state_iter) = iter_op {
            if let Some(updated_state) = enclave_context.update_state(updated_state_iter) {
                output.update(updated_state);
            }
        }

        let roster_idx = self.ciphertext().roster_idx() as usize;
        // ratchet app keychain per a log.
        group_key.ratchet(roster_idx)?;

        Ok(output)
    }
}

impl EcallHandler for input::InsertHandshake {
    type O = output::Empty;

    fn handle<R, C>(
        self,
        enclave_context: &C,
        _max_mem_size: usize
    ) -> Result<Self::O>
    where
        R: RuntimeExecutor<C, S=StateType>,
        C: ContextOps<S=StateType> + Clone,
    {
        let group_key = &mut *enclave_context.write_group_key();
        let handshake = HandshakeParams::decode(&mut self.handshake())
            .map_err(|_| anyhow!("HandshakeParams::decode Error"))?;

        group_key.process_handshake(&handshake)?;

        Ok(output::Empty::default())
    }
}

impl EcallHandler for input::GetState {
    type O = output::ReturnState;

    fn handle<R, C>(
        self,
        enclave_context: &C,
        _max_mem_size: usize
    ) -> Result<Self::O>
    where
        R: RuntimeExecutor<C, S=StateType>,
        C: ContextOps<S=StateType> + Clone,
    {
        let addr = self.access_right().verified_user_address()?;
        let user_state = enclave_context.get_state(addr, self.mem_id());

        Ok(output::ReturnState::new(user_state))
    }
}

pub fn inner_ecall_join_group(
    raw_join_group_tx: &mut RawJoinGroupTx,
    enclave_context: &EnclaveContext,
    ias_url: &str,
    test_sub_key: &str,
) -> Result<()> {
    let join_group_tx = JoinGroupTx::construct(
        ias_url,
        test_sub_key,
        &enclave_context,
    )?;
    *raw_join_group_tx = join_group_tx.into_raw()?;

    Ok(())
}

pub fn inner_ecall_handshake(
    raw_handshake_tx: &mut RawHandshakeTx,
    enclave_context: &EnclaveContext,
) -> Result<()> {
    let handshake_tx = HandshakeTx::construct(&enclave_context)?;
    *raw_handshake_tx = handshake_tx.into_raw()?;

    Ok(())
}

pub fn inner_ecall_register_notification(
    sig: &RawSig,
    pubkey: &RawPubkey,
    challenge: &RawChallenge,
    enclave_context: &EnclaveContext,
) -> Result<()> {
    let sig = Signature::from_bytes(&sig[..])
        .map_err(|e| anyhow!("{}", e))?;
    let pubkey = PublicKey::from_bytes(&pubkey[..])
        .map_err(|e| anyhow!("{}", e))?;
    let user_address = UserAddress::from_sig(&challenge[..], &sig, &pubkey)
        .map_err(|e| anyhow!("{}", e))?;

    enclave_context.set_notification(user_address);

    Ok(())
}
