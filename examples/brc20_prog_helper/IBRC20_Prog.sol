// SPDX-License-Identifier: MIT

pragma solidity ^0.8.19;

/**
 * @dev Interface for the BRC-20 Prog helper functions.
 */
interface IBRC20_Prog {
    /**
     * @dev Verifies BIP322 signature, given address, message and the signature.
     */
    function verifyBIP322Signature(
        string calldata addr,
        string calldata message_base64,
        string calldata signature_base64
    ) external view returns (bool verified);

    /**
     * @dev Get non-module BRC-20 balance of a given Bitcoin wallet script and BRC-20 ticker.
     */
    function getBrc20BalanceOf(
        string calldata ticker,
        string calldata address_pkscript
    ) external view returns (uint256 balance);

    /**
     * @dev Get Bitcoin transaction details using tx ids.
     */
    function getTxDetails(
        string calldata txid
    )
        external
        view
        returns (
            uint256 block_height,
            string[] memory vin_txids,
            uint256[] memory vin_vouts,
            string[] memory vin_scriptPubKey_hexes,
            uint256[] memory vin_values,
            string[] memory vout_scriptPubKey_hexes,
            uint256[] memory vout_values
        );

    /**
     * @dev Get last satoshi location of a given sat location in a transaction.
     */
    function getLastSatLocation(
        string calldata txid,
        uint256 vout,
        uint256 sat
    ) external view returns (string memory last_txid, uint256 last_vout, uint256 last_sat, string memory old_pkscript, string memory new_pkscript);

    /**
     * @dev Get locked pkscript of a given Bitcoin wallet script.
     */
    function getLockedPkscript(
        string calldata address_pkscript,
        uint256 lock_block_count
    ) external view returns (string memory locked_pkscript);
}
