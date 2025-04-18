use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;
use std::path::Path;

use cached_database::BlockDatabase;
use revm::context::result::ExecutionResult;
use revm::context::DBErrorMarker;
use revm::primitives::alloy_primitives::{Bloom, U128, U64};
use revm::primitives::ruint::aliases::U256;
use revm::primitives::{Address, Bytes, FixedBytes, B256};
use revm::{Database as DatabaseTrait, DatabaseCommit};
use revm_state::{Account, AccountInfo, Bytecode};

mod cached_database;
use cached_database::{BlockCachedDatabase, BlockHistoryCacheData};

pub mod types;

use rs_merkle::algorithms::Sha256;
use rs_merkle::MerkleTree;
pub use types::{
    AccountInfoED, AddressED, BEncodeDecode, BlockResponseED, BytecodeED, LogResponseED, TxED,
    TxReceiptED, UintEncodeDecode, B256ED, U128ED, U256ED, U512ED, U64ED,
};

pub const MAX_HISTORY_SIZE: u64 = 10;

pub struct DB {
    /// Account address to memory location
    /// TODO: If the value is zero, consider deleting it from the database to save space
    db_account_memory: Option<BlockCachedDatabase<U512ED, U256ED, BlockHistoryCacheData<U256ED>>>,

    /// Code hash to bytecode
    db_code: Option<BlockCachedDatabase<B256ED, BytecodeED, BlockHistoryCacheData<BytecodeED>>>,

    /// Account address to account info
    db_account:
        Option<BlockCachedDatabase<AddressED, AccountInfoED, BlockHistoryCacheData<AccountInfoED>>>,

    /// Block number and index to tx hash
    db_number_and_index_to_tx_hash:
        Option<BlockCachedDatabase<U128ED, B256ED, BlockHistoryCacheData<B256ED>>>,

    /// TxHash to tx receipt
    db_tx_receipt:
        Option<BlockCachedDatabase<B256ED, TxReceiptED, BlockHistoryCacheData<TxReceiptED>>>,

    /// Tx hash to Tx
    db_tx: Option<BlockCachedDatabase<B256ED, TxED, BlockHistoryCacheData<TxED>>>,

    /// Hash of Inscription IDs to TxHash
    db_inscription_id_to_tx_hash:
        Option<BlockCachedDatabase<String, B256ED, BlockHistoryCacheData<B256ED>>>,

    /// Block hash to block number
    db_block_hash_to_number:
        Option<BlockCachedDatabase<B256ED, U64ED, BlockHistoryCacheData<U64ED>>>,

    // Block number to Block
    db_block_number_to_block: Option<BlockDatabase<BlockResponseED>>,

    /// Block number to block hash
    db_block_number_to_hash: Option<BlockDatabase<B256ED>>,

    /// Block number to block timestamp
    db_block_number_to_timestamp: Option<BlockDatabase<U64ED>>,

    /// Block number to gas used
    db_block_number_to_gas_used: Option<BlockDatabase<U64ED>>,

    /// Block number to mine timestamp
    db_block_number_to_mine_tm: Option<BlockDatabase<U128ED>>,

    /// Cache for latest block number and block hash
    latest_block_number: Option<(u64, B256)>,
}

impl Default for DB {
    fn default() -> Self {
        Self {
            db_account_memory: None,
            db_code: None,
            db_account: None,
            db_number_and_index_to_tx_hash: None,
            db_tx_receipt: None,
            db_tx: None,
            db_inscription_id_to_tx_hash: None,
            db_block_number_to_block: None,
            db_block_number_to_hash: None,
            db_block_hash_to_number: None,
            db_block_number_to_timestamp: None,
            db_block_number_to_gas_used: None,
            db_block_number_to_mine_tm: None,
            latest_block_number: None,
        }
    }
}

impl DB {
    pub fn new(base_path: &Path) -> Result<Self, Box<dyn Error>> {
        rlimit::Resource::NOFILE.set(4096, 8192)?;

        Ok(Self {
            db_account_memory: Some(BlockCachedDatabase::new(&base_path, "account_memory")),
            db_code: Some(BlockCachedDatabase::new(&base_path, "code")),
            db_account: Some(BlockCachedDatabase::new(&base_path, "account")),
            db_number_and_index_to_tx_hash: Some(BlockCachedDatabase::new(
                &base_path,
                "number_and_index_to_tx_hash",
            )),
            db_tx_receipt: Some(BlockCachedDatabase::new(&base_path, "tx_receipt")),
            db_inscription_id_to_tx_hash: Some(BlockCachedDatabase::new(
                &base_path,
                "inscription_id_to_tx_hash",
            )),
            db_tx: Some(BlockCachedDatabase::new(&base_path, "tx")),
            db_block_hash_to_number: Some(BlockCachedDatabase::new(
                &base_path,
                "block_hash_to_number",
            )),
            db_block_number_to_block: Some(BlockDatabase::new(&base_path, "block_number_to_block")),
            db_block_number_to_hash: Some(BlockDatabase::new(&base_path, "block_number_to_hash")),
            db_block_number_to_timestamp: Some(BlockDatabase::new(
                &base_path,
                "block_number_to_timestamp",
            )),
            db_block_number_to_gas_used: Some(BlockDatabase::new(
                &base_path,
                "block_number_to_gas_used",
            )),
            db_block_number_to_mine_tm: Some(BlockDatabase::new(
                &base_path,
                "block_number_to_mine_tm",
            )),
            latest_block_number: None,
        })
    }

    pub fn get_latest_block_height(&self) -> Result<u64, Box<dyn Error>> {
        if self.latest_block_number.is_some() {
            return Ok(self.latest_block_number.unwrap().0);
        }
        Ok(self
            .db_block_number_to_hash
            .as_ref()
            .unwrap()
            .last_key()?
            .unwrap_or(0))
    }

    pub fn get_account_memory(
        &mut self,
        account: Address,
        mem_loc: U256,
    ) -> Result<Option<U256ED>, Box<dyn Error>> {
        let ret = self
            .db_account_memory
            .as_ref()
            .unwrap()
            .latest(&U512ED::from_addr_u256(account, mem_loc))?;

        Ok(ret)
    }

    pub fn set_account_memory(
        &mut self,
        account: Address,
        mem_loc: U256,
        value: U256,
    ) -> Result<(), Box<dyn Error>> {
        let block_number = self.get_latest_block_height()?;
        self.db_account_memory.as_mut().unwrap().set(
            block_number,
            U512ED::from_addr_u256(account, mem_loc),
            U256ED::from_u256(value),
        )?;

        Ok(())
    }

    pub fn get_code(&mut self, code_hash: B256) -> Result<Option<BytecodeED>, Box<dyn Error>> {
        let ret = self
            .db_code
            .as_ref()
            .unwrap()
            .latest(&B256ED::from_b256(code_hash))?;

        Ok(ret)
    }

    pub fn set_code(&mut self, code_hash: B256, bytecode: Bytecode) -> Result<(), Box<dyn Error>> {
        let block_number = self.get_latest_block_height()?;
        Ok(self.db_code.as_mut().unwrap().set(
            block_number,
            B256ED::from_b256(code_hash),
            BytecodeED(bytecode),
        )?)
    }

    fn get_number_and_index_key(block_number: u64, tx_idx: u64) -> u128 {
        ((block_number as u128) << 64) | tx_idx as u128
    }

    pub fn get_logs(
        &mut self,
        mut block_number_from: Option<u64>,
        mut block_number_to: Option<u64>,
        contract_address: Option<Address>,
        topics: Vec<B256>,
    ) -> Result<Vec<LogResponseED>, Box<dyn Error>> {
        if block_number_from.is_none() {
            block_number_from = self.latest_block_number.map(|x| x.0);
        }
        if block_number_to.is_none() {
            block_number_to = self.latest_block_number.map(|x| x.0);
        }

        // Limit the number of blocks to be fetched
        // DB is under a lock here and if this takes too long, it will block other threads
        //
        // TODO: This is a temporary solution, we can potentially avoid using a mutex for reads
        // TODO: Also, test this, maybe it's not that slow?
        if block_number_to.unwrap() - block_number_from.unwrap() > 5 {
            return Ok(Vec::new());
        }

        let mut logs = Vec::new();

        let tx_ids = self
            .db_number_and_index_to_tx_hash
            .as_ref()
            .unwrap()
            .get_range(
                &U128ED::from_u128(Self::get_number_and_index_key(
                    block_number_from.unwrap(),
                    0,
                )),
                &&U128ED::from_u128(Self::get_number_and_index_key(
                    block_number_to.unwrap() + 1,
                    0,
                )),
            )?;

        for tx_pair in tx_ids {
            let tx_id = tx_pair.1;
            let tx_receipt = self.get_tx_receipt(tx_id.0).unwrap().unwrap();
            if tx_receipt.contract_address.is_none() {
                continue;
            }
            if contract_address.is_some()
                && tx_receipt.contract_address.unwrap().0 != contract_address.unwrap()
            {
                continue;
            }

            let mut current_index = tx_receipt.logs.log_index;
            for log in tx_receipt.logs.logs {
                let mut matched = true;
                if topics.len() != 0 && log.topics().len() != topics.len() {
                    current_index += 1;
                    continue;
                }

                for (i, topic) in topics.iter().enumerate() {
                    if log.topics()[i] != *topic {
                        matched = false;
                        break;
                    }
                }

                if matched {
                    logs.push(LogResponseED {
                        address: AddressED(log.address),
                        topics: log
                            .topics()
                            .iter()
                            .map(|x: &FixedBytes<32>| B256ED::from_b256(*x))
                            .collect(),
                        data: log.data.data,
                        transaction_index: U64ED::from_u64(tx_receipt.transaction_index),
                        transaction_hash: tx_receipt.transaction_hash.clone(),
                        block_hash: tx_receipt.hash.clone(),
                        block_number: U64ED::from_u64(tx_receipt.block_number),
                        log_index: U64ED::from_u64(current_index),
                    });
                }

                current_index += 1;
            }
        }

        Ok(logs)
    }

    pub fn get_tx_count(
        &mut self,
        account: Option<Address>,
        block_number: u64,
    ) -> Result<u64, Box<dyn Error>> {
        let tx_ids = self
            .db_number_and_index_to_tx_hash
            .as_ref()
            .unwrap()
            .get_range(
                &U128ED::from_u128(Self::get_number_and_index_key(block_number, 0)),
                &&U128ED::from_u128(Self::get_number_and_index_key(block_number + 1, 0)),
            )?;

        let mut count = 0;
        for tx_pair in tx_ids {
            let tx_id = tx_pair.1;
            let tx = self.get_tx_by_hash(tx_id.0).unwrap().unwrap();
            if account.is_none() || tx.from.0 == account.unwrap() {
                count += 1;
            }
        }
        Ok(count)
    }

    pub fn get_tx_hash_by_inscription_id(
        &mut self,
        inscription_id: String,
    ) -> Result<Option<B256ED>, Box<dyn Error>> {
        let ret = self
            .db_inscription_id_to_tx_hash
            .as_ref()
            .unwrap()
            .latest(&inscription_id)?;

        Ok(ret)
    }

    pub fn set_tx_hash_by_inscription_id(
        &mut self,
        inscription_id: String,
        tx_hash: B256,
    ) -> Result<(), Box<dyn Error>> {
        let block_number = self.get_latest_block_height()?;
        Ok(self.db_inscription_id_to_tx_hash.as_mut().unwrap().set(
            block_number,
            inscription_id,
            B256ED::from_b256(tx_hash),
        )?)
    }

    pub fn get_tx_hash_by_block_number_and_index(
        &mut self,
        block_number: u64,
        tx_idx: u64,
    ) -> Result<Option<B256ED>, Box<dyn Error>> {
        let key = Self::get_number_and_index_key(block_number, tx_idx);
        let ret = self
            .db_number_and_index_to_tx_hash
            .as_ref()
            .unwrap()
            .latest(&U128ED::from_u128(key))?;

        Ok(ret)
    }

    pub fn get_tx_hash_by_block_hash_and_index(
        &mut self,
        block_hash: B256,
        tx_idx: u64,
    ) -> Result<Option<B256ED>, Box<dyn Error>> {
        let block_number = self.get_block_number(block_hash)?;
        if block_number.is_none() {
            return Ok(None);
        }

        self.get_tx_hash_by_block_number_and_index(block_number.unwrap().to_u64(), tx_idx)
    }

    pub fn get_tx_by_hash(&mut self, tx_hash: B256) -> Result<Option<TxED>, Box<dyn Error>> {
        let ret = self
            .db_tx
            .as_ref()
            .unwrap()
            .latest(&B256ED::from_b256(tx_hash))?;

        Ok(ret)
    }

    pub fn get_tx_receipt(&mut self, tx_hash: B256) -> Result<Option<TxReceiptED>, Box<dyn Error>> {
        let ret = self
            .db_tx_receipt
            .as_ref()
            .unwrap()
            .latest(&B256ED::from_b256(tx_hash))?;

        Ok(ret)
    }

    pub fn set_tx_receipt(
        &mut self,
        result_type: &str,
        reason: &str,
        result: Option<&Bytes>,
        block_hash: B256,
        block_number: u64,
        block_timestamp: u64,
        contract_address: Option<Address>,
        from: Address,
        to: Option<Address>,
        data: &Bytes,
        tx_hash: B256,
        tx_idx: u64,
        output: &ExecutionResult,
        cumulative_gas_used: u64,
        nonce: u64,
        start_log_index: u64,
        inscription_id: Option<String>,
    ) -> Result<(), Box<dyn Error>> {
        let tx_receipt = TxReceiptED::new(
            block_hash,
            block_number,
            block_timestamp,
            contract_address,
            from,
            to,
            tx_hash,
            tx_idx,
            output,
            cumulative_gas_used,
            nonce,
            start_log_index,
            result_type.to_string(),
            reason.to_string(),
            result,
        );

        let tx = TxED {
            hash: B256ED::from_b256(tx_hash),
            nonce,
            block_hash: B256ED::from_b256(block_hash),
            block_number,
            transaction_index: tx_idx,
            from: AddressED(from),
            to: to.map(AddressED),
            value: 0,
            gas: 0,
            gas_price: 0,
            input: data.clone(),
        };

        self.db_tx
            .as_mut()
            .unwrap()
            .set(block_number, B256ED::from_b256(tx_hash), tx)?;

        self.db_number_and_index_to_tx_hash.as_mut().unwrap().set(
            block_number,
            U128ED::from_u128(Self::get_number_and_index_key(block_number, tx_idx)),
            B256ED::from_b256(tx_hash),
        )?;

        if inscription_id.is_some() {
            self.set_tx_hash_by_inscription_id(inscription_id.unwrap(), tx_hash)?;
        }

        Ok(self.db_tx_receipt.as_mut().unwrap().set(
            block_number,
            B256ED::from_b256(tx_hash),
            tx_receipt,
        )?)
    }

    pub fn get_account_info(
        &mut self,
        account: Address,
    ) -> Result<Option<AccountInfoED>, Box<dyn Error>> {
        let ret = self
            .db_account
            .as_ref()
            .unwrap()
            .latest(&AddressED(account))?;

        Ok(ret)
    }

    pub fn set_account_info(
        &mut self,
        account: Address,
        value: AccountInfo,
    ) -> Result<(), Box<dyn Error>> {
        let block_number = self.get_latest_block_height()?;
        Ok(self.db_account.as_mut().unwrap().set(
            block_number,
            AddressED(account),
            AccountInfoED(value),
        )?)
    }

    pub fn get_block(
        &mut self,
        block_number: u64,
    ) -> Result<Option<BlockResponseED>, Box<dyn Error>> {
        let block_hash = self.get_block_hash(block_number)?;
        if block_hash.is_none() {
            return Ok(None);
        }

        let stored_block = self
            .db_block_number_to_block
            .as_mut()
            .unwrap()
            .get(block_number)?;
        if stored_block.is_some() {
            return Ok(Some(stored_block.unwrap()));
        }

        let block_hash = block_hash.unwrap();
        let block_timestamp = self.get_block_timestamp(block_number)?;
        let gas_used = self.get_gas_used(block_number)?;
        let mine_timestamp = self.get_mine_timestamp(block_number)?;

        let parent_hash = if block_number == 0 {
            B256::ZERO
        } else {
            self.get_block_hash(block_number - 1)?
                .unwrap_or(FixedBytes([0u8; 32]))
        };

        let tx_ids = self
            .db_number_and_index_to_tx_hash
            .as_ref()
            .unwrap()
            .get_range(
                &U128ED::from_u128(Self::get_number_and_index_key(block_number, 0)),
                &&U128ED::from_u128(Self::get_number_and_index_key(block_number + 1, 0)),
            )?;

        let leaves = tx_ids.iter().map(|x| x.1 .0 .0).collect::<Vec<[u8; 32]>>();

        let tx_merkle = MerkleTree::<Sha256>::from_leaves(leaves.as_slice());

        let mut transactions = Vec::new();
        let mut bloom = Bloom::new([0u8; 256]);
        for tx_pair in tx_ids {
            let tx_id = tx_pair.1;
            let tx = self.get_tx_receipt(tx_id.0).unwrap();
            if tx.is_some() {
                for log in tx.unwrap().logs.logs {
                    bloom.accrue_log(&log);
                }
            }
            transactions.push(tx_id);
        }

        let block_response = BlockResponseED::new(
            0,
            36000000,
            gas_used.unwrap_or(U64::ZERO).as_limbs()[0],
            BEncodeDecode(block_hash),
            BEncodeDecode(FixedBytes(bloom.as_slice().try_into().unwrap())),
            transactions.len() as u64,
            block_number,
            block_timestamp.unwrap_or(U64::ZERO).as_limbs()[0],
            UintEncodeDecode(mine_timestamp.unwrap_or(U128::ZERO)),
            transactions,
            BEncodeDecode(FixedBytes(tx_merkle.root().unwrap_or([0; 32]))),
            0,
            BEncodeDecode(parent_hash),
            BEncodeDecode(FixedBytes([0; 32])),
            0,
        );

        self.db_block_number_to_block
            .as_mut()
            .unwrap()
            .set(block_number, block_response.clone());

        Ok(Some(block_response))
    }

    pub fn get_block_number(&mut self, block_hash: B256) -> Result<Option<U64ED>, Box<dyn Error>> {
        let ret = self
            .db_block_hash_to_number
            .as_ref()
            .unwrap()
            .latest(&B256ED::from_b256(block_hash))?;

        Ok(ret)
    }

    pub fn get_block_hash(&mut self, block_number: u64) -> Result<Option<B256>, Box<dyn Error>> {
        let ret = self
            .db_block_number_to_hash
            .as_mut()
            .unwrap()
            .get(block_number)?;

        Ok(ret.map(|x| x.0))
    }

    pub fn set_block_hash(
        &mut self,
        block_number: u64,
        mut block_hash: B256,
    ) -> Result<(), Box<dyn Error>> {
        if self.latest_block_number.is_none()
            || block_number > self.latest_block_number.unwrap_or((0, B256::ZERO)).0
        {
            self.latest_block_number = Some((block_number, block_hash));
        }

        if block_hash == B256::ZERO {
            // just hash the number
            let bytes = block_number.to_be_bytes();
            let full_bytes = [0u8; 24]
                .iter()
                .chain(bytes.iter())
                .copied()
                .collect::<Vec<u8>>();
            block_hash = B256::from_slice(&full_bytes);
        }

        self.db_block_number_to_hash
            .as_mut()
            .unwrap()
            .set(block_number, B256ED::from_b256(block_hash));

        Ok(self.db_block_hash_to_number.as_mut().unwrap().set(
            block_number,
            B256ED::from_b256(block_hash),
            U64ED::from_u64(block_number),
        )?)
    }

    pub fn get_block_timestamp(&mut self, number: u64) -> Result<Option<U64>, Box<dyn Error>> {
        let ret = self
            .db_block_number_to_timestamp
            .as_mut()
            .unwrap()
            .get(number)?;

        Ok(ret.map(|x| x.0))
    }

    pub fn set_block_timestamp(
        &mut self,
        block_number: u64,
        block_timestamp: u64,
    ) -> Result<(), Box<dyn Error>> {
        Ok(self
            .db_block_number_to_timestamp
            .as_mut()
            .unwrap()
            .set(block_number, U64ED::from_u64(block_timestamp)))
    }

    pub fn get_gas_used(&mut self, block_number: u64) -> Result<Option<U64>, Box<dyn Error>> {
        let ret = self
            .db_block_number_to_gas_used
            .as_mut()
            .unwrap()
            .get(block_number)?;

        Ok(ret.map(|x| x.0))
    }

    pub fn set_gas_used(&mut self, block_number: u64, gas_used: u64) -> Result<(), Box<dyn Error>> {
        Ok(self
            .db_block_number_to_gas_used
            .as_mut()
            .unwrap()
            .set(block_number, U64ED::from_u64(gas_used)))
    }

    pub fn get_mine_timestamp(
        &mut self,
        block_number: u64,
    ) -> Result<Option<U128>, Box<dyn Error>> {
        let ret = self
            .db_block_number_to_mine_tm
            .as_mut()
            .unwrap()
            .get(block_number)?;

        Ok(ret.map(|x| x.0))
    }

    pub fn set_mine_timestamp(
        &mut self,
        block_number: u64,
        mine_timestamp: u128,
    ) -> Result<(), Box<dyn Error>> {
        Ok(self
            .db_block_number_to_mine_tm
            .as_mut()
            .unwrap()
            .set(block_number, U128ED::from_u128(mine_timestamp)))
    }

    pub fn commit_changes(&mut self) -> Result<(), Box<dyn Error>> {
        let latest_block_number = self.get_latest_block_height()?;

        self.db_block_number_to_hash.as_mut().unwrap().commit()?;
        self.db_block_number_to_timestamp
            .as_mut()
            .unwrap()
            .commit()?;
        self.db_block_number_to_gas_used
            .as_mut()
            .unwrap()
            .commit()?;
        self.db_block_number_to_mine_tm.as_mut().unwrap().commit()?;
        self.db_block_number_to_block.as_mut().unwrap().commit()?;

        self.db_number_and_index_to_tx_hash
            .as_mut()
            .unwrap()
            .commit(latest_block_number)?;
        self.db_inscription_id_to_tx_hash
            .as_mut()
            .unwrap()
            .commit(latest_block_number)?;
        self.db_tx.as_mut().unwrap().commit(latest_block_number)?;
        self.db_tx_receipt
            .as_mut()
            .unwrap()
            .commit(latest_block_number)?;
        self.db_account_memory
            .as_mut()
            .unwrap()
            .commit(latest_block_number)?;
        self.db_code.as_mut().unwrap().commit(latest_block_number)?;
        self.db_account
            .as_mut()
            .unwrap()
            .commit(latest_block_number)?;
        self.db_block_hash_to_number
            .as_mut()
            .unwrap()
            .commit(latest_block_number)?;

        self.clear_caches();
        Ok(())
    }

    pub fn clear_caches(&mut self) {
        self.db_account_memory.as_mut().unwrap().clear_cache();
        self.db_code.as_mut().unwrap().clear_cache();
        self.db_account.as_mut().unwrap().clear_cache();
        self.db_block_number_to_hash.as_mut().unwrap().clear_cache();
        self.db_block_hash_to_number.as_mut().unwrap().clear_cache();
        self.db_inscription_id_to_tx_hash
            .as_mut()
            .unwrap()
            .clear_cache();
        self.db_tx.as_mut().unwrap().clear_cache();
        self.db_tx_receipt.as_mut().unwrap().clear_cache();
        self.db_number_and_index_to_tx_hash
            .as_mut()
            .unwrap()
            .clear_cache();
        self.db_block_number_to_timestamp
            .as_mut()
            .unwrap()
            .clear_cache();
        self.db_block_number_to_gas_used
            .as_mut()
            .unwrap()
            .clear_cache();
        self.db_block_number_to_mine_tm
            .as_mut()
            .unwrap()
            .clear_cache();
        self.db_block_number_to_block
            .as_mut()
            .unwrap()
            .clear_cache();

        self.latest_block_number = None;
    }

    pub fn reorg(&mut self, latest_valid_block_number: u64) -> Result<(), Box<dyn Error>> {
        self.db_account_memory
            .as_mut()
            .unwrap()
            .reorg(latest_valid_block_number)?;
        self.db_code
            .as_mut()
            .unwrap()
            .reorg(latest_valid_block_number)?;
        self.db_account
            .as_mut()
            .unwrap()
            .reorg(latest_valid_block_number)?;
        self.db_block_hash_to_number
            .as_mut()
            .unwrap()
            .reorg(latest_valid_block_number)?;
        self.db_number_and_index_to_tx_hash
            .as_mut()
            .unwrap()
            .reorg(latest_valid_block_number)?;
        self.db_tx_receipt
            .as_mut()
            .unwrap()
            .reorg(latest_valid_block_number)?;
        self.db_inscription_id_to_tx_hash
            .as_mut()
            .unwrap()
            .reorg(latest_valid_block_number)?;
        self.db_tx
            .as_mut()
            .unwrap()
            .reorg(latest_valid_block_number)?;

        self.db_block_number_to_hash
            .as_mut()
            .unwrap()
            .reorg(latest_valid_block_number)?;
        self.db_block_number_to_timestamp
            .as_mut()
            .unwrap()
            .reorg(latest_valid_block_number)?;
        self.db_block_number_to_gas_used
            .as_mut()
            .unwrap()
            .reorg(latest_valid_block_number)?;
        self.db_block_number_to_mine_tm
            .as_mut()
            .unwrap()
            .reorg(latest_valid_block_number)?;
        self.db_block_number_to_block
            .as_mut()
            .unwrap()
            .reorg(latest_valid_block_number)?;

        Ok(self.commit_changes()?)
    }
}

#[derive(Debug)]
pub struct DBError(Box<dyn Error>);

impl DBErrorMarker for DBError {}

impl Display for DBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DBError: {}", self.0)
    }
}

impl Error for DBError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&*self.0)
    }
}

impl DatabaseTrait for DB {
    type Error = DBError;

    /// Get basic account information.
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let res = self.get_account_info(address);
        res.map(|x| {
            x.map(|x| {
                let mut account_info = x.0;
                account_info.code = Some(self.code_by_hash(account_info.code_hash).unwrap());
                account_info
            })
        })
        .map_err(|x| DBError(x))
    }

    /// Get account code by its hash.
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        self.get_code(code_hash)
            .map(|x| x.unwrap_or(BytecodeED(Bytecode::new())).0)
            .map_err(|x| DBError(x))
    }

    /// Get storage value of address at index.
    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        self.get_account_memory(address, index)
            .map(|x| x.unwrap_or(U256ED::from_u256(U256::ZERO)).0)
            .map_err(|x| DBError(x))
    }

    /// Get block hash by block number.
    fn block_hash(&mut self, number: u64) -> Result<B256, Self::Error> {
        self.get_block_hash(number)
            .map(|x| x.unwrap_or(B256::ZERO))
            .map_err(|x| DBError(x))
    }
}

impl DatabaseCommit for DB {
    fn commit(
        &mut self,
        changes: HashMap<
            alloy_primitives::Address,
            Account,
            alloy_primitives::map::foldhash::fast::RandomState,
        >,
    ) {
        for (address, account) in changes {
            if !account.is_touched() {
                continue;
            }
            let mut acc_info = AccountInfo::default();
            acc_info.balance = account.info.balance;
            acc_info.nonce = account.info.nonce;
            acc_info.code_hash = account.info.code_hash;
            let _ = self.set_account_info(address, acc_info);

            let is_newly_created = account.is_created();
            if is_newly_created {
                // TODO: can contract change other than creation??
                let _ = self.set_code(account.info.code_hash, account.info.code.unwrap());
            }

            for (loc, slot) in account.storage {
                if !slot.is_changed() {
                    continue;
                }
                let _ = self.set_account_memory(address, loc, slot.present_value());
            }
        }
    }
}

/// Tests for all set and get methods
#[cfg(test)]
mod tests {
    use revm::context::result::{Output, SuccessReason};
    use revm::primitives::{Address, Bytes};
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_db() {
        let path = TempDir::new().unwrap().into_path();

        let address = Address::from([1u8; 20]);
        let code_hash = B256::from([2u8; 32]);
        let bytecode = Bytecode::new_raw(Bytes::from(vec![3u8; 32]));
        let account_info = AccountInfo {
            balance: U256::from(100),
            nonce: 4,
            code_hash,
            code: Some(bytecode.clone()),
        };
        let mine_timestamp = 5;

        let mem_loc = U256::from(6);
        let value = U256::from(7);
        let block_number = 8;
        let block_hash = B256::from([9u8; 32]);
        let block_timestamp = 10;
        let gas_used = 11;

        {
            let mut db = DB::new(&path).unwrap();

            db.set_account_info(address, account_info.clone()).unwrap();
            assert_eq!(
                db.get_account_info(address).unwrap().unwrap().0,
                account_info
            );

            db.set_code(code_hash, bytecode.clone()).unwrap();
            assert_eq!(db.get_code(code_hash).unwrap().unwrap().0, bytecode);

            db.set_account_memory(address, mem_loc, value).unwrap();
            assert_eq!(
                db.get_account_memory(address, mem_loc).unwrap().unwrap().0,
                value
            );

            db.set_block_hash(block_number, block_hash).unwrap();
            assert_eq!(
                db.get_block_hash(block_number).unwrap().unwrap(),
                block_hash
            );

            db.set_block_timestamp(block_number, block_timestamp)
                .unwrap();
            assert_eq!(
                db.get_block_timestamp(block_number).unwrap().unwrap(),
                block_timestamp.try_into().unwrap()
            );

            db.set_gas_used(block_number, gas_used).unwrap();
            assert_eq!(
                db.get_gas_used(block_number).unwrap().unwrap(),
                gas_used.try_into().unwrap()
            );

            db.set_mine_timestamp(block_number, mine_timestamp).unwrap();
            assert_eq!(
                db.get_mine_timestamp(block_number).unwrap().unwrap(),
                mine_timestamp.try_into().unwrap()
            );

            db.commit_changes().unwrap();
        }

        let mut db = DB::new(&path).unwrap();

        assert_eq!(
            db.get_account_info(address).unwrap().unwrap().0,
            account_info
        );
        assert_eq!(
            db.get_code(code_hash).unwrap().unwrap().0.bytes(),
            bytecode.bytes()
        );
        assert_eq!(
            db.get_account_memory(address, mem_loc).unwrap().unwrap().0,
            value
        );
        assert_eq!(
            db.get_block_hash(block_number).unwrap().unwrap(),
            block_hash
        );
        assert_eq!(
            db.get_block_timestamp(block_number).unwrap().unwrap(),
            block_timestamp.try_into().unwrap()
        );
        assert_eq!(
            db.get_gas_used(block_number).unwrap().unwrap(),
            gas_used.try_into().unwrap()
        );
        assert_eq!(
            db.get_mine_timestamp(block_number).unwrap().unwrap(),
            mine_timestamp.try_into().unwrap()
        );
    }

    #[test]
    fn test_tx_methods() {
        let path = TempDir::new().unwrap().into_path();

        let data = vec![0u8; 32];
        let block_hash = B256::from([1u8; 32]);
        let block_number = 2;
        let block_timestamp = 11;
        let contract_address = Address::from([3u8; 20]);
        let from = Address::from([4u8; 20]);
        let to = Address::from([5u8; 20]);
        let tx_hash = B256::from([6u8; 32]);
        let tx_idx = 7;
        let output = ExecutionResult::Success {
            reason: SuccessReason::Return,
            gas_used: 10,
            gas_refunded: 0,
            logs: Vec::new(),
            output: Output::Call(Bytes::from(vec![11u8; 32])),
        };
        let cumulative_gas_used = 8;
        let nonce = 9;
        let start_log_index = 10;

        {
            let mut db = DB::new(&path).unwrap();

            db.set_block_hash(block_number, block_hash).unwrap();
            db.set_tx_receipt(
                "type",
                "reason",
                Some(&Bytes::from(vec![11u8; 32])),
                block_hash,
                block_number,
                block_timestamp,
                Some(contract_address),
                from,
                Some(to),
                &Bytes::from(data),
                tx_hash,
                tx_idx,
                &output,
                cumulative_gas_used,
                nonce,
                start_log_index,
                Some("inscription_id".to_string()),
            )
            .unwrap();

            db.commit_changes().unwrap();
        }

        let mut db = DB::new(&path).unwrap();

        assert_eq!(
            db.get_tx_hash_by_inscription_id("inscription_id".to_string())
                .unwrap()
                .unwrap()
                .0,
            tx_hash
        );
        assert_eq!(
            db.get_tx_hash_by_block_number_and_index(block_number, tx_idx)
                .unwrap()
                .unwrap()
                .0,
            tx_hash
        );
        assert_eq!(
            db.get_tx_hash_by_block_hash_and_index(block_hash, tx_idx)
                .unwrap()
                .unwrap()
                .0,
            tx_hash
        );
        assert_eq!(
            db.get_tx_receipt(tx_hash).unwrap().unwrap(),
            TxReceiptED::new(
                block_hash,
                block_number,
                block_timestamp,
                Some(contract_address),
                from,
                Some(to),
                tx_hash,
                tx_idx,
                &output,
                cumulative_gas_used,
                nonce,
                start_log_index,
                "type".to_string(),
                "reason".to_string(),
                Some(&Bytes::from(vec![11u8; 32])),
            )
        );
    }
}
