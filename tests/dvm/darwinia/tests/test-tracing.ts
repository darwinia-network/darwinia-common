import { expect } from "chai";
import Web3 from "web3";
import { config } from "./config";
import { contractFile } from "./contract/compile";
import { customRequest } from "./utils";

const web3 = new Web3(config.host);
const bytecode = contractFile.evm.bytecode.object;
const abi = contractFile.abi;
const incrementer = new web3.eth.Contract(abi);

describe("Test Evm Tracing", function () {
	let create_contract;
	let transaction_hash;
	let trace_result = null;

	it.skip("Test debug_traceTransaction(Deploy Contract)", async function () {
		const incrementerTx = incrementer.deploy({
			data: bytecode,
			arguments: [5],
		});

		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: config.address,
				data: incrementerTx.encodeABI(),
				gas: config.gas,
			},
			config.privKey
		);

		const createReceipt = await web3.eth.sendSignedTransaction(
			createTransaction.rawTransaction
		);
		create_contract = createReceipt.contractAddress;

		// debug_traceTransaction for create transaction
		transaction_hash = createReceipt.transactionHash;
		trace_result = await customRequest("debug_traceTransaction", [transaction_hash]);
		expect(trace_result.result.stepLogs.length).to.be.equal(61);
		const block_number = web3.utils.toHex(createReceipt.blockNumber);

		// debug_traceTransaction with callTracer
		trace_result = await customRequest("debug_traceBlockByNumber", [
			block_number,
			{ tracer: "callTracer" },
		]);
		expect(trace_result.result[0].from).to.be.equal(config.address.toLowerCase());
		expect(trace_result.result[0].to).to.be.equal(create_contract.toLowerCase());
		expect(trace_result.result[0].type).to.be.equal("CREATE");
	}).timeout(80000);

	it.skip("Test debug_traceBlock(Call Contract)", async function () {
		const value = 3;
		const encoded = incrementer.methods.increment(value).encodeABI();
		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: config.address,
				to: create_contract,
				data: encoded,
				gas: config.gas,
			},
			config.privKey
		);

		const createReceipt = await web3.eth.sendSignedTransaction(
			createTransaction.rawTransaction
		);
		// debug_traceTransaction for call transaction
		transaction_hash = createReceipt.transactionHash;
		trace_result = await customRequest("debug_traceTransaction", [transaction_hash]);
		expect(trace_result.result.stepLogs.length).to.be.equal(69);
		expect(trace_result.result.stepLogs[0].depth).to.be.equal(1);
		expect(trace_result.result.stepLogs[0].pc).to.be.equal(0);

		// debug_traceBlockByNumber
		const block_number = web3.utils.toHex(createReceipt.blockNumber);
		trace_result = await customRequest("debug_traceBlockByNumber", [
			block_number,
			{ tracer: "callTracer" },
		]);
		expect(trace_result.result[0].from).to.be.equal(config.address.toLowerCase());
		expect(trace_result.result[0].to).to.be.equal(create_contract.toLowerCase());
		expect(trace_result.result[0].type).to.be.equal("CALL");

		// debug_traceBlockByHash
		const block_hash = createReceipt.blockHash;
		trace_result = await customRequest("debug_traceBlockByNumber", [
			block_hash,
			{ tracer: "callTracer" },
		]);
		expect(trace_result.result[0].from).to.be.equal(config.address.toLowerCase());
		expect(trace_result.result[0].to).to.be.equal(create_contract.toLowerCase());
		expect(trace_result.result[0].type).to.be.equal("CALL");
	}).timeout(80000);

	it("Test trace correctly transfer", async () => {
		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: config.address,
				to: "0xAa01a1bEF0557fa9625581a293F3AA7770192632",
				data: "0x0",
				gas: config.gas,
				value: "0x10000000",
			},
			config.privKey
		);

		const createReceipt = await web3.eth.sendSignedTransaction(
			createTransaction.rawTransaction
		);

		transaction_hash = createReceipt.transactionHash;
		trace_result = await customRequest("debug_traceTransaction", [transaction_hash]);
		console.log("trace result: ", trace_result);
		expect(trace_result.result.gas).to.be.eq("0x5208"); // 21_000 gas for a transfer.
	}).timeout(80000);

    
});
