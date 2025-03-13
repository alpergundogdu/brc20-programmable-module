use std::str::FromStr;

use db::DB;
use revm::{
    precompile::Error, primitives::{Bytes, PrecompileErrors, PrecompileOutput, PrecompileResult, B256}, ContextStatefulPrecompile
};
use solabi::{selector, FunctionEncoder, U256};

lazy_static::lazy_static! {
    static ref BTC_CLIENT: reqwest::blocking::Client = reqwest::blocking::Client::new();
    static ref BITCOIN_RPC_URL: String = std::env::var("BITCOIN_RPC_URL")
            .unwrap_or("http://localhost:48332".to_string());
    static ref BITCOIN_RPC_USER: String = std::env::var("BITCOIN_RPC_USER")
            .unwrap_or("user".to_string());
    static ref BITCOIN_RPC_PASSWORD: String = std::env::var("BITCOIN_RPC_PASSWORD")
            .unwrap_or("password".to_string());
}

static INITIAL_GAS: u64 = 50000;
static GAS_PER_RPC_CALL: u64 = 100000;

pub struct BTCPrecompile;

/// Signature for the getTxDetails function in the BTCPrecompile contract
/// Uses get raw tx details from the blockchain using the json rpc and returns the details from the transaction
/// ScriptPubKey for the vin transaction is fetched using the txid and vout
///
/// # Returns (block_height, vin_txid, vin_vout, vin_scriptPubKey_hex, vin_value, vout_scriptPubKey_hex, vout_value) in a tuple
/// # Errors - Returns an error if the transaction details are not found
const TX_DETAILS: FunctionEncoder<String, (U256, Vec<String>, Vec<U256>, Vec<String>, Vec<U256>, Vec<String>, Vec<U256>)> =
    FunctionEncoder::new(selector!("getTxDetails(string)"));

impl ContextStatefulPrecompile<DB> for BTCPrecompile {
    fn call(
        &self,
        bytes: &Bytes,
        gas_limit: u64,
        _evmctx: &mut revm::InnerEvmContext<DB>,
    ) -> PrecompileResult {
        let gas_used = INITIAL_GAS + GAS_PER_RPC_CALL;
        if gas_used > gas_limit {
            return Err(PrecompileErrors::Error(Error::OutOfGas));
        }

        let result = TX_DETAILS.decode_params(&bytes);

        if result.is_err() {
            return Err(PrecompileErrors::Error(Error::Other(
                "Invalid params".to_string(),
            )));
        }

        let txid = result.unwrap();

        let response = get_raw_transaction(&txid);

        if response["error"].is_object() {
            return Err(PrecompileErrors::Error(Error::Other(
                response["error"]["message"].as_str().unwrap().to_string(),
            )));
        }


        let response = response["result"].clone();

        let vin_count = response["vin"].as_array();
        if gas_used + (vin_count.unwrap().len() as u64 * GAS_PER_RPC_CALL) > gas_limit {
            return Err(PrecompileErrors::Error(Error::OutOfGas));
        }

        let block_hash = response["blockhash"].as_str().unwrap_or("").to_string();
        let block_height = _evmctx
            .db
            .get_block_number(B256::from_str(&block_hash).unwrap_or(B256::ZERO))
            .unwrap()
            .map(|x| x.0.as_limbs()[0])
            .unwrap_or(0);

        let mut vin_txids = Vec::new();
        let mut vin_vouts = Vec::new();
        let mut vin_script_pub_key_hexes = Vec::new();
        let mut vin_values = Vec::new();
        let mut vout_script_pub_key_hexes = Vec::new();
        let mut vout_values = Vec::new();

        for vin in response["vin"].as_array().unwrap().into_iter() {
            let vin_txid = vin["txid"].as_str().unwrap_or("").to_string();
            let vin_vout = vin["vout"].as_u64().unwrap_or(0);

            // Get the scriptPubKey from the vin transaction, using the txid and vout
            let vin_script_pub_key_response = get_raw_transaction(&vin_txid);
            if vin_script_pub_key_response["error"].is_object() {
                return Err(PrecompileErrors::Error(Error::Other(
                    vin_script_pub_key_response["error"]["message"]
                        .as_str()
                        .unwrap()
                        .to_string(),
                )));
            }

            let vin_script_pub_key_response = vin_script_pub_key_response["result"].clone();
            let vin_script_pub_key_hex = vin_script_pub_key_response["vout"][vin_vout as usize]
                ["scriptPubKey"]["hex"]
                .as_str()
                .unwrap_or("")
                .to_string();

            let vin_value = vin_script_pub_key_response["vout"][vin_vout as usize]["value"]
                .as_f64()
                .unwrap_or(0.0);
            let vin_value = (vin_value * 100000000.0) as u64;

            vin_txids.push(vin_txid);
            vin_vouts.push(U256::from(vin_vout));
            vin_script_pub_key_hexes.push(vin_script_pub_key_hex);
            vin_values.push(U256::from(vin_value));
        }

        for vout in response["vout"].as_array().unwrap().into_iter() {
            let vout_script_pub_key_hex = vout["scriptPubKey"]["hex"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let vout_value = vout["value"].as_f64().unwrap_or(0.0);
            let vout_value = (vout_value * 100000000.0) as u64;
            
            vout_script_pub_key_hexes.push(vout_script_pub_key_hex);
            vout_values.push(U256::from(vout_value));
        }

        let bytes = TX_DETAILS.encode_returns(&(
            U256::from(block_height),
            vin_txids,
            vin_vouts,
            vin_script_pub_key_hexes,
            vin_values,
            vout_script_pub_key_hexes,
            vout_values,
        ));

        Ok(PrecompileOutput {
            bytes: Bytes::from(bytes),
            gas_used,
        })
    }
}

fn get_raw_transaction(txid: &str) -> serde_json::Value {
    let response = BTC_CLIENT
        .post(&*BITCOIN_RPC_URL)
        .basic_auth(&*BITCOIN_RPC_USER, Some(&*BITCOIN_RPC_PASSWORD))
        .body(
            format!(
                "{{
                \"jsonrpc\": \"1.0\",
                \"id\": \"b2p\",
                \"method\": \"getrawtransaction\",
                \"params\": {{\"txid\":\"{}\", \"verbose\": true}}
                }}",
                txid
            )
            .to_string(),
        )
        .send()
        .unwrap();

    response.json().unwrap()
}

#[cfg(test)]
mod tests {
    use solabi::U256;

    use super::TX_DETAILS;

    #[test]
    fn test_get_tx_details_encode_params() {
        let txid = "ab6baebc91d645aade178f952bf75e62735b37ee692717e090b1b1f2a2b243ba";
        let data = TX_DETAILS.encode_params(&txid.to_string());
        assert_eq!(
            hex::encode(data),
            "96327323000000000000000000000000000000000000000000000000000000000000004061623662616562633931643634356161646531373866393532626637356536323733356233376565363932373137653039306231623166326132623234336261"
        );
    }

    #[test]
    fn test_get_tx_details_decode_returns() {
        // https://mempool.space/testnet4/tx/ce1d2d142eb12fa4fbbb2c361c286483e5c74ca67640496de23beb5ee56d0406
        let data = "0000000000000000000000000000000000000000000000000000000000011d7600000000000000000000000000000000000000000000000000000000000000e0000000000000000000000000000000000000000000000000000000000000028000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000500000000000000000000000000000000000000000000000000000000000000058000000000000000000000000000000000000000000000000000000000000007e00000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000012000000000000000000000000000000000000000000000000000000000000000403162613331333332323662313866313865636339653333333539613063356131323433323033613734666334666466383839386639653730396138323630616100000000000000000000000000000000000000000000000000000000000000406561326135353337343733336433633336313432373235623366343538353762663464373862323931353365613464636466386162356238383062343934663800000000000000000000000000000000000000000000000000000000000000406233663138663062343139656335653435323731363737373633343930656637626532653662346264353531396462613932346564316333316537383764346400000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000e00000000000000000000000000000000000000000000000000000000000000160000000000000000000000000000000000000000000000000000000000000004435313230353436656231386135643435396262353964393637396665386638643539386662663735363862663035636464613361663662323631386238666438633366340000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000443531323035343665623138613564343539626235396439363739666538663864353938666266373536386266303563646461336166366232363138623866643863336634000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000044353132303534366562313861356434353962623539643936373966653866386435393866626637353638626630356364646133616636623236313862386664386333663400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000000000000000002220000000000000000000000000000000000000000000000000000000000000222000000000000000000000000000000000000000000000000000000000012cb220000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000014000000000000000000000000000000000000000000000000000000000000001c0000000000000000000000000000000000000000000000000000000000000001c36613564306231363031303065393964303431656633383436613032000000000000000000000000000000000000000000000000000000000000000000000044353132303534366562313861356434353962623539643936373966653866386435393866626637353638626630356364646133616636623236313862386664386333663400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004435313230623430633036356266636335393632653137303266303964653161356432646663306137323336626261663563313637323532396234313462336565346366350000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000443531323035343665623138613564343539626235396439363739666538663864353938666266373536386266303563646461336166366232363138623866643863336634000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002220000000000000000000000000000000000000000000000000000000000000222000000000000000000000000000000000000000000000000000000000012b211";
        let (
            block_height,
            vin_txids,
            vin_vouts,
            vin_script_pub_key_hexes,
            vin_values,
            vout_script_pub_key_hexes,
            vout_values,
        ) = TX_DETAILS
            .decode_returns(hex::decode(data).unwrap().as_slice())
            .unwrap();

        assert_eq!(block_height, U256::from(73078u64));
        assert_eq!(vin_txids.len(), 3);
        assert_eq!(vin_vouts.len(), 3);
        assert_eq!(vin_script_pub_key_hexes.len(), 3);
        assert_eq!(vin_values.len(), 3);

        assert_eq!(vout_script_pub_key_hexes.len(), 4);
        assert_eq!(vout_values.len(), 4);

        assert_eq!(vin_txids[0], "1ba3133226b18f18ecc9e33359a0c5a1243203a74fc4fdf8898f9e709a8260aa");
        assert_eq!(vin_vouts[0], U256::from(2u64));
        assert_eq!(vin_script_pub_key_hexes[0], "5120546eb18a5d459bb59d9679fe8f8d598fbf7568bf05cdda3af6b2618b8fd8c3f4");
        assert_eq!(vin_values[0], U256::from(546u64));

        assert_eq!(vin_txids[1], "ea2a55374733d3c36142725b3f45857bf4d78b29153ea4dcdf8ab5b880b494f8");
        assert_eq!(vin_vouts[1], U256::from(2u64));
        assert_eq!(vin_script_pub_key_hexes[1], "5120546eb18a5d459bb59d9679fe8f8d598fbf7568bf05cdda3af6b2618b8fd8c3f4");
        assert_eq!(vin_values[1], U256::from(546u64));

        assert_eq!(vin_txids[2], "b3f18f0b419ec5e45271677763490ef7be2e6b4bd5519dba924ed1c31e787d4d");
        assert_eq!(vin_vouts[2], U256::from(0u64));
        assert_eq!(vin_script_pub_key_hexes[2], "5120546eb18a5d459bb59d9679fe8f8d598fbf7568bf05cdda3af6b2618b8fd8c3f4");
        assert_eq!(vin_values[2], U256::from(1231650u64));

        assert_eq!(vout_script_pub_key_hexes[0], "6a5d0b160100e99d041ef3846a02");
        assert_eq!(vout_values[0], U256::from(0u64));
        assert_eq!(vout_script_pub_key_hexes[1], "5120546eb18a5d459bb59d9679fe8f8d598fbf7568bf05cdda3af6b2618b8fd8c3f4");
        assert_eq!(vout_values[1], U256::from(546u64));
        assert_eq!(vout_script_pub_key_hexes[2], "5120b40c065bfcc5962e1702f09de1a5d2dfc0a7236bbaf5c1672529b414b3ee4cf5");
        assert_eq!(vout_values[2], U256::from(546u64));
        assert_eq!(vout_script_pub_key_hexes[3], "5120546eb18a5d459bb59d9679fe8f8d598fbf7568bf05cdda3af6b2618b8fd8c3f4");
        assert_eq!(vout_values[3], U256::from(1225233u64));
    }
}
