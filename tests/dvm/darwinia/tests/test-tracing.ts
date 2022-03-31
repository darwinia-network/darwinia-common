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
	let filter_begin,
		filter_end = null;

	it("Test trace transfer(Raw)", async () => {
		const createTransaction = await web3.eth.accounts.signTransaction(
			{
				from: config.address,
				to: "0xAa01a1bEF0557fa9625581a293F3AA7770192632",
				data: "0x",
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
	}).timeout(20000);

	it("Test Deploy Contract(Raw, Call)", async function () {
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

		// debug_traceTransaction with RawTracer
		transaction_hash = createReceipt.transactionHash;
		trace_result = await customRequest("debug_traceTransaction", [transaction_hash]);
		expect(trace_result.result.stepLogs.length).to.be.equal(61);
		const block_number = web3.utils.toHex(createReceipt.blockNumber);

		filter_begin = block_number;

		// debug_traceTransaction with callTracer
		trace_result = await customRequest("debug_traceBlockByNumber", [
			block_number,
			{ tracer: "callTracer" },
		]);
		expect(trace_result.result[0].from).to.be.equal(config.address.toLowerCase());
		expect(trace_result.result[0].to).to.be.equal(create_contract.toLowerCase());
		expect(trace_result.result[0].type).to.be.equal("CREATE");
	}).timeout(20000);

	it("Test Call Contract(Raw, Call)", async function () {
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
		// debug_traceTransaction with RawTracer
		transaction_hash = createReceipt.transactionHash;
		trace_result = await customRequest("debug_traceTransaction", [transaction_hash]);
		expect(trace_result.result.stepLogs.length).to.be.equal(69);
		expect(trace_result.result.stepLogs[0].depth).to.be.equal(1);
		expect(trace_result.result.stepLogs[0].pc).to.be.equal(0);

		// debug_traceBlockByNumber with CallTracer
		const block_number = web3.utils.toHex(createReceipt.blockNumber);

		filter_end = block_number;

		trace_result = await customRequest("debug_traceBlockByNumber", [
			block_number,
			{ tracer: "callTracer" },
		]);
		expect(trace_result.result[0].from).to.be.equal(config.address.toLowerCase());
		expect(trace_result.result[0].to).to.be.equal(create_contract.toLowerCase());
		expect(trace_result.result[0].type).to.be.equal("CALL");

		// debug_traceBlockByHash with CallTracer
		const block_hash = createReceipt.blockHash;
		trace_result = await customRequest("debug_traceBlockByNumber", [
			block_hash,
			{ tracer: "callTracer" },
		]);
		expect(trace_result.result[0].from).to.be.equal(config.address.toLowerCase());
		expect(trace_result.result[0].to).to.be.equal(create_contract.toLowerCase());
		expect(trace_result.result[0].type).to.be.equal("CALL");
	}).timeout(20000);

	it("Test trace_filter rpc works", async function () {
		trace_result = await customRequest("trace_filter", [
			{
				fromBlock: filter_begin,
				toBlock: filter_end,
				fromAddress: [config.address],
			},
		]);
		expect(trace_result.result.length).to.equal(2);

		trace_result = await customRequest("trace_filter", [
			{
				fromBlock: filter_begin,
				toBlock: filter_end,
				fromAddress: [config.address],
				after: 1,
			},
		]);
		expect(trace_result.result.length).to.equal(1);
	}).timeout(20000);
});
