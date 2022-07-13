use crate::error::Error;
use alloc::{vec, vec::Vec};
use ckb_std::ckb_constants::Source;
use ckb_std::ckb_types::packed::CellOutput;
use ckb_std::high_level::{load_cell, load_transaction, load_witness_args, QueryIter};
use ckb_std::{
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_script, load_tx_hash},
};
use core::result::Result;
use molecule::hex_string;
use rhai::plugin::CallableFunction::Script;
use rhai::{Engine, Scope};
use serde::{Deserialize, Serialize};
use serde_json_core::to_string;

#[derive(Debug, Serialize, Deserialize)]
struct Script<'a> {
    pub code_hash: &'a str,
    pub hash_type: u8,
    pub args: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
struct Cell<'a> {
    pub capacity: u64,
    pub lock: Script<'a>,
    pub type_: Option<Script<'a>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Tx<'a> {
    inputs: Vec<Cell<'a>>,
    outputs: Vec<Cell<'a>>,
}

pub fn main() -> Result<(), Error> {
    let witness_args = load_witness_args(0, Source::Input)?;
    if let Some(witness_args_type) = witness_args.input_type().to_opt() {
        let witness_args_input_type: Bytes = witness_args_type.unpack();

        let inputs: Vec<Cell> = QueryIter::new(load_cell, Source::Input)
            .map(parse_cell)
            .collect();
        let outputs: Vec<Cell> = QueryIter::new(load_cell, Source::Output)
            .map(parse_cell)
            .collect();
        let tx = Tx { inputs, outputs };
        let tx_json = to_string(&tx).map_err(|_| Error::Encoding)?;
        let engine = Engine::new();
        let tx_map = engine.parse_json(tx_json, true)?;
        let mut scope = Scope::new();
        scope.push("tx", tx_map);

        let extension_script = hex_string(&witness_args_input_type.to_vec());
        let result = engine
            .eval_with_scope::<bool>(&mut scope, &extension_script)
            .map_err(|_| Error::RunExtensionScriptError)?;

        if !result {
            return Err(Error::RunExtensionScriptError);
        }
    }
    Err(Error::WitnessArgsError)
}

fn parse_cell(cell_output: CellOutput) -> Cell {
    Cell {
        capacity: cell_output.capacity().unpack(),
        lock: Script {
            code_hash: &hex_string(cell_output.lock().code_hash().as_slice()),
            hash_type: u8::from_be_bytes(cell_output.lock().hash_type().into()),
            args: &hex_string(cell_output.lock().args().as_slice()),
        },
        type_: cell_output.type_().to_opt().map(|t| Script {
            code_hash: &hex_string(t.code_hash().as_slice()),
            hash_type: u8::from_be_bytes(t.hash_type().into()),
            args: &hex_string(t.args().as_slice()),
        }),
    }
}
