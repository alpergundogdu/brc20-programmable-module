var solc_fixed = require('solc_0_8_24');
var ethers = require('ethers');
var fs = require('fs');

function findImports(path) {
    if (!fs.existsSync(path)) {
        path = 'node_modules/' + path
    }
    if (!fs.existsSync(path)) {
        return { error: 'File not found' };
    }
    return {
        contents:
            fs.readFileSync(path, 'utf8')
    };
}

async function main() {
    let BRC20Prog_sol = fs.readFileSync("./BRC20_Prog.sol", "utf8");

    let input = {
        language: 'Solidity',
        sources: {
            'BRC20_Prog.sol': { content: BRC20Prog_sol },
        },
        settings: {
            evmVersion: "cancun",
            outputSelection: {
                '*': {
                    '*': ['*']
                }
            }
        }
    };

    let compiled = JSON.parse(solc_fixed.compile(JSON.stringify(input), { import: findImports }))
    let contract = compiled.contracts['BRC20_Prog.sol']["BRC20_Prog"]
    let bytecode = contract.evm.bytecode.object
    let abi = contract.abi

    let contract_factory = new ethers.ContractFactory(abi, bytecode, null)
    let deploy_tx = await contract_factory.getDeployTransaction()

    fs.mkdirSync('output', { recursive: true });

    fs.writeFile('output/BRC20_Prog.abi', JSON.stringify(abi, null, 4), function (_) { });
    fs.writeFile('output/BRC20_Prog.bytecode', bytecode, function (_) { });

    fs.writeFile('output/BRC20_Prog_deploy_tx.json', JSON.stringify(
        {
            p: "brc20-prog",
            op: "deploy",
            d: deploy_tx.data
        }
    ), function (_) { });

    let address = "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l"
    let message = "Hello World"
    let signature = "AkgwRQIhAOzyynlqt93lOKJr+wmmxIens//zPzl9tqIOua93wO6MAiBi5n5EyAcPScOjf1lAqIUIQtr3zKNeavYabHyR8eGhowEhAsfxIAMZZEKUPYWI4BruhAQjzFT8FSFSajuFwrDL1Yhy"

    fs.writeFile('output/BRC20_Prog_bip322_verify_tx.json', JSON.stringify(
        {
            p: "brc20-prog",
            op: "call",
            c: "REPLACE_THIS_WITH_CONTRACT_ADDRESS",
            d: contract_factory.interface.encodeFunctionData("verifyBIP322Signature", [address, message, signature]),
        }
    ), function (_) { });

    let btc_pkscript = "bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l"
    let ticker = "bleh"

    fs.writeFile('output/BRC20_Prog_brc20_balance_tx.json', JSON.stringify(
        {
            p: "brc20-prog",
            op: "call",
            c: "REPLACE_THIS_WITH_CONTRACT_ADDRESS",
            d: contract_factory.interface.encodeFunctionData("getBrc20BalanceOf", [ticker, btc_pkscript]),
        }
    ), function (_) { });

    // https://mempool.space/signet/tx/4183fb733b9553ca8b93208c91dda18bee3d0b8510720b15d76d979af7fd9926
    let btc_tx_id = "4183fb733b9553ca8b93208c91dda18bee3d0b8510720b15d76d979af7fd9926"

    fs.writeFile('output/BRC20_Prog_btc_tx_details_tx.json', JSON.stringify(
        {
            p: "brc20-prog",
            op: "call",
            c: "REPLACE_THIS_WITH_CONTRACT_ADDRESS",
            d: contract_factory.interface.encodeFunctionData("getTxDetails", [btc_tx_id]),
        }
    ), function (_) { });

    let btc_vout = 2
    let btc_sat = 250000

    fs.writeFile('output/BRC20_Prog_btc_last_sat_loc_tx.json', JSON.stringify(
        {
            p: "brc20-prog",
            op: "call",
            c: "REPLACE_THIS_WITH_CONTRACT_ADDRESS",
            d: contract_factory.interface.encodeFunctionData("getLastSatLocation", [btc_tx_id, btc_vout, btc_sat]),
        }
    ), function (_) { });

    let lock_block_count = 100

    fs.writeFile('output/BRC20_Prog_btc_locked_pkscript_tx.json', JSON.stringify(
        {
            p: "brc20-prog",
            op: "call",
            c: "REPLACE_THIS_WITH_CONTRACT_ADDRESS",
            d: contract_factory.interface.encodeFunctionData("getLockedPkscript", [btc_tx_id, lock_block_count]),
        }
    ), function (_) { });
}

main()