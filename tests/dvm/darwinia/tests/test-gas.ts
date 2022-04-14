import { expect } from "chai";
import Web3 from "web3";
import { config, EXTRINSIC_GAS_LIMIT } from "./config";
import { customRequest } from "./utils";
import { contractFile } from "./contract/compile";

const web3 = new Web3(config.host);
const bytecode = contractFile.evm.bytecode.object;
const abi = contractFile.abi;
const incrementer = new web3.eth.Contract(abi);

describe("Test Gas", function () {
	const incrementerTx = incrementer.deploy({
		data: bytecode,
		arguments: [5],
	});

    it("Test block gas limit 1", async function () {
		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: config.address,
				data: incrementerTx.encodeABI(),
				gas: EXTRINSIC_GAS_LIMIT - 1,
			},
			config.privKey
		);

		const createReceipt = await web3.eth.sendSignedTransaction(
			createTransaction.rawTransaction
		);
		expect(createReceipt.transactionHash).to.be.not.null;
		expect(createReceipt.blockHash).to.be.not.null;
	}).timeout(20000);

    it.skip("Test block gas limit 2", async function () {
		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: config.address,
				data: incrementerTx.encodeABI(),
				gas: EXTRINSIC_GAS_LIMIT,
			},
			config.privKey
		);

        let result = await customRequest("eth_sendRawTransaction", [createTransaction.rawTransaction]);
        console.log(result);
        expect((result as any).error.message).to.equal("submit transaction to pool failed: Pool(TemporarilyBanned)");
		
	}).timeout(20000);

	

	
});

// 0.025 * 1000