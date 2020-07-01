use std::{
    path::Path,
    io::BufReader,
    fs::File,
    str::FromStr,
    marker::PhantomData,
};
use web3::types::Address;
use ethabi::Contract as ContractABI;
use anonify_runtime::{
    traits::{State, CallNameConverter},
};
use anyhow::anyhow;
use crate::{
    error::Result,
    eth::primitives::Web3Contract,
};

/// Needed information to handle smart contracts.
#[derive(Debug, Clone, Copy)]
pub struct ContractInfo<'a, P: AsRef<Path>> {
    abi_path: P,
    addr: &'a str,
}

impl<'a, P: AsRef<Path>> ContractInfo<'a, P> {
    pub fn new(abi_path: P, addr: &'a str) -> Self {
        ContractInfo {
            abi_path,
            addr,
        }
    }

    pub fn contract_abi(&self) -> Result<ContractABI> {
        let f = File::open(&self.abi_path)?;
        let reader = BufReader::new(f);

        ContractABI::load(reader)
            .map_err(|e| anyhow!("Failed to load contract abi.: {:?}", e))
            .map_err(Into::into)
    }

    pub fn address(&self) -> Result<Address> {
        Address::from_str(self.addr)
            .map_err(|e| anyhow!("{:?}", e))
            .map_err(Into::into)
    }
}

#[derive(Debug, Clone)]
pub struct StateInfo<'a, ST: State, C: CallNameConverter> {
    state: ST,
    state_id: u64,
    call_name: &'a str,
    phantom: PhantomData<C>
}

impl<'a, ST: State, C: CallNameConverter> StateInfo<'a, ST, C> {
    pub fn new(state: ST, state_id: u64, call_name: &'a str) -> Self {
        StateInfo {
            state,
            state_id,
            call_name,
            phantom: PhantomData::<C>,
        }
    }

    pub fn call_name_to_id(&self) -> u32 {
        C::as_id(self.call_name)
    }

    pub fn state_as_bytes(&self) -> Vec<u8> {
        self.state.as_bytes()
    }

    pub fn state_id(&self) -> u64 {
        self.state_id
    }
}

/// A type of transaction signing address
#[derive(Debug, Clone)]
pub enum SignerAddress {
    EthAddress(web3::types::Address)
}

/// A type of contract
pub enum ContractKind {
    Web3Contract(Web3Contract)
}
