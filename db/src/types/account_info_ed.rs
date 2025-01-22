use std::{borrow::Cow, error::Error};

use heed::{BytesDecode, BytesEncode};
use revm::primitives::{AccountInfo, B256, U256, ruint::aliases::U64};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AccountInfoED(pub AccountInfo);

impl AccountInfoED {
    pub fn from_account_info(a: AccountInfo) -> Self {
        Self(a)
    }
}

impl<'a> BytesEncode<'a> for AccountInfoED {
    type EItem = AccountInfoED;

    fn bytes_encode(item: &'a Self::EItem) -> Result<Cow<'a, [u8]>, Box<dyn Error>> {
        let mut bytes = Vec::new();
        for limb in item.0.balance.as_limbs().iter() {
            bytes.extend_from_slice(&limb.to_be_bytes());
        }
        bytes.extend_from_slice(&item.0.nonce.to_be_bytes());
        bytes.extend_from_slice(&item.0.code_hash.0.to_vec());
        Ok(Cow::Owned(bytes))
    }
}

impl<'a> BytesDecode<'a> for AccountInfoED {
    type DItem = AccountInfoED;

    fn bytes_decode(bytes: &'a [u8]) -> Result<Self::DItem, Box<dyn Error>> {
        let balance = U256::from_be_bytes::<32>(bytes[0..32].try_into().unwrap());
        let nonce = U64::from_be_bytes::<8>(bytes[32..40].try_into().unwrap()).try_into().unwrap();
        let code_hash_u = U256::from_be_bytes::<32>(bytes[40..72].try_into().unwrap());
        let code_hash = B256::from(code_hash_u);
        Ok(AccountInfoED(AccountInfo {
            balance,
            nonce,
            code_hash,
            code: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use heed::{BytesDecode, BytesEncode};
    use revm::primitives::AccountInfo;
    use revm::primitives::B256;
    use revm::primitives::U256;

    use crate::types::AccountInfoED;

    #[test]
    fn test_account_info_ed() {
        // test by converting to bytes and decoding
        let account_info = AccountInfoED::from_account_info(AccountInfo {
            balance: U256::from(100),
            nonce: 1,
            code_hash: B256::from([1; 32]),
            code: None,
        });
        let bytes = AccountInfoED::bytes_encode(&account_info).unwrap();
        let decoded = AccountInfoED::bytes_decode(&bytes).unwrap();
        assert_eq!(account_info.0.balance, decoded.0.balance);
        assert_eq!(account_info.0.nonce, decoded.0.nonce);
        assert_eq!(account_info.0.code_hash, decoded.0.code_hash);
        assert_eq!(account_info.0.code, decoded.0.code);
    }
}
