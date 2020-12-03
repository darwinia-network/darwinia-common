const expect = require('chai').expect;
const Web3 = require('web3');

const web3 = new Web3('http://localhost:9933');

const addressFrom = '0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b';
// substrate: '5ELRpquT7C3mWtjeqFMYqgNbcNgWKSr3mYtVi1Uvtc2R7YEx';
const privKey =
    '99B3C12287537E38C90A9219D4CB074A89A16E9CDB20BF85728EBD97C343E342';

// ```
// pragma solidity >=0.4.22 <0.7.0;
//
// contract WillFail {
//		 constructor() public {
//				 require(false);
//		 }
// }
// ```
const FAIL_BYTECODE = '6080604052348015600f57600080fd5b506000601a57600080fd5b603f8060276000396000f3fe6080604052600080fdfea26469706673582212209f2bb2a4cf155a0e7b26bd34bb01e9b645a92c82e55c5dbdb4b37f8c326edbee64736f6c63430006060033';
const GOOD_BYTECODE = '6080604052348015600f57600080fd5b506001601a57600080fd5b603f8060276000396000f3fe6080604052600080fdfea2646970667358221220c70bc8b03cdfdf57b5f6c4131b836f9c2c4df01b8202f530555333f2a00e4b8364736f6c63430006060033';


describe('Test RPC Bloom', function () {
    it("should provide a tx receipt after successful deployment", async function () {

        const createTransaction = await web3.eth.accounts.signTransaction(
            {
                from: addressFrom,
                data: GOOD_BYTECODE,
                value: "0x00",
                gasPrice: "0x01",
                gas: "0x100000",
            },
            privKey
        );

        const tx = await web3.eth.sendSignedTransaction(createTransaction.rawTransaction);

        const receipt = await web3.eth.getTransactionReceipt(tx.transactionHash);
        expect(receipt).to.include({
            contractAddress: '0xfE5D3c52F7ee9aa32a69b96Bfbb088Ba0bCd8EfC',
            cumulativeGasUsed: 67231,
            from: '0x6be02d1d3665660d22ff9624b7be0551ee1ac91b',
            gasUsed: 67231,
            to: null,
            transactionHash: tx.transactionHash,
            transactionIndex: 0,
            status: true
        });
    }).timeout(10000);

    it("should provide a tx receipt after failed deployment", async function () {

        const createTransaction = await web3.eth.accounts.signTransaction(
            {
                from: addressFrom,
                data: FAIL_BYTECODE,
                value: "0x00",
                gasPrice: "0x01",
                gas: "0x100000",
            },
            privKey
        );

        const tx = await waitForHash(createTransaction.rawTransaction);

        await delay(9000);
        const receipt = await web3.eth.getTransactionReceipt(tx.result);
        expect(receipt).to.include({
            contractAddress: '0x92496871560a01551E1B4fD04540D7A519D5C19e',
            cumulativeGasUsed: 54600,
            from: '0x6be02d1d3665660d22ff9624b7be0551ee1ac91b',
            gasUsed: 54600,
            to: null,
            transactionHash: tx.result,
            transactionIndex: 0,
            status: false
        });
    }).timeout(100000);
});

function waitForHash(signedTx) {
    return new Promise((resolve, reject) => {
        web3.currentProvider.send({
            method: "eth_sendRawTransaction",
            params: [signedTx],
            jsonrpc: "2.0",
            id: 1
        }, function (error, result) {
            if (error) {
                console.log("bear: error", error);
                reject(
                    `Failed to send custom request (${method} (${params.join(",")})): ${
                        error.message || error.toString()
                    }`
                );
            }
            resolve(result)
        })
    })
}

function delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}