import { expect } from "chai";

import Test from "../build/contracts/Test.json"
import { createAndFinalizeBlock, createAndFinalizeBlockNowait, customRequest, describeWithFrontier } from "./util";
import { AbiItem } from "web3-utils";

describeWithFrontier("Frontier RPC (Contract Methods)", (context) => {
	const GENESIS_ACCOUNT = "0x6be02d1d3665660d22ff9624b7be0551ee1ac91b";
	const GENESIS_ACCOUNT_PRIVATE_KEY = "0x99B3C12287537E38C90A9219D4CB074A89A16E9CDB20BF85728EBD97C343E342";

	const TEST_CONTRACT_BYTECODE = Test.bytecode;
	const TEST_CONTRACT_ABI = Test.abi as AbiItem[];
	const FIRST_CONTRACT_ADDRESS = "0xc2bf5f29a4384b1ab0c063e1c666f02121b6084a"; // Those test are ordered. In general this should be avoided, but due to the time it takes	// to spin up a frontier node, it saves a lot of time.

	before("create the contract", async function () {
		this.timeout(15000);
		const tx = await context.web3.eth.accounts.signTransaction(
			{
				from: GENESIS_ACCOUNT,
				data: TEST_CONTRACT_BYTECODE,
				value: "0x00",
				gasPrice: "0x3B9ACA00",
				gas: "0x100000",
			},
			GENESIS_ACCOUNT_PRIVATE_KEY
		);
		await customRequest(context.web3, "eth_sendRawTransaction", [tx.rawTransaction]);
		await createAndFinalizeBlock(context.web3);
	});

	it("get transaction by hash", async () => {
		const latestBlock = await context.web3.eth.getBlock("latest");
		expect(latestBlock.transactions.length).to.equal(1);

		const tx_hash = latestBlock.transactions[0];
		const tx = await context.web3.eth.getTransaction(tx_hash);
		expect(tx.hash).to.equal(tx_hash);
	});

	it("should return contract method result", async function () {
		const contract = new context.web3.eth.Contract(TEST_CONTRACT_ABI, FIRST_CONTRACT_ADDRESS, {
			from: GENESIS_ACCOUNT,
			gasPrice: "0x3B9ACA00",
		});

		expect(await contract.methods.multiply(3).call()).to.equal("21");
	});
	it("should get correct environmental block number", async function () {
		// Solidity `block.number` is expected to return the same height at which the runtime call was made.
		const contract = new context.web3.eth.Contract(TEST_CONTRACT_ABI, FIRST_CONTRACT_ADDRESS, {
			from: GENESIS_ACCOUNT,
			gasPrice: "0x3B9ACA00",
		});
		let block = await context.web3.eth.getBlock("latest");
		expect(await contract.methods.currentBlock().call()).to.eq(block.number.toString());
		await createAndFinalizeBlock(context.web3);
		block = await context.web3.eth.getBlock("latest");
		expect(await contract.methods.currentBlock().call()).to.eq(block.number.toString());
	});

	it("should get correct environmental block hash", async function () {
		this.timeout(20000);
		// Solidity `blockhash` is expected to return the ethereum block hash at a given height.
		const contract = new context.web3.eth.Contract(TEST_CONTRACT_ABI, FIRST_CONTRACT_ADDRESS, {
			from: GENESIS_ACCOUNT,
			gasPrice: "0x3B9ACA00",
		});
		let number = (await context.web3.eth.getBlock("latest")).number;
		let last = number + 256;
		for(let i = number; i <= last; i++) {
			let hash = (await context.web3.eth.getBlock("latest")).hash;
			expect(await contract.methods.blockHash(i).call()).to.eq(hash);
			await createAndFinalizeBlockNowait(context.web3);
		}
		// should not store more than 256 hashes
		expect(await contract.methods.blockHash(number).call()).to.eq(
			"0x0000000000000000000000000000000000000000000000000000000000000000"
		);
	});

	it("should get correct environmental block gaslimit", async function () {
		const contract = new context.web3.eth.Contract(TEST_CONTRACT_ABI, FIRST_CONTRACT_ADDRESS, {
			from: GENESIS_ACCOUNT,
			gasPrice: "0x3B9ACA00",
		});
		// Max u32
		expect(await contract.methods.gasLimit().call()).to.eq('4294967295');
	});

	// Requires error handling
	it.skip("should fail for missing parameters", async function () {
		const contract = new context.web3.eth.Contract([{ ...TEST_CONTRACT_ABI[0], inputs: [] }], FIRST_CONTRACT_ADDRESS, {
			from: GENESIS_ACCOUNT,
			gasPrice: "0x3B9ACA00",
		});
		await contract.methods
			.multiply()
			.call()
			.catch((err) =>
				expect(err.message).to.equal(`Returned error: VM Exception while processing transaction: revert.`)
			);
	});

	// Requires error handling
	it.skip("should fail for too many parameters", async function () {
		const contract = new context.web3.eth.Contract(
			[
				{
					...TEST_CONTRACT_ABI[0],
					inputs: [
						{ internalType: "uint256", name: "a", type: "uint256" },
						{ internalType: "uint256", name: "b", type: "uint256" },
					],
				},
			],
			FIRST_CONTRACT_ADDRESS,
			{
				from: GENESIS_ACCOUNT,
				gasPrice: "0x3B9ACA00",
			}
		);
		await contract.methods
			.multiply(3, 4)
			.call()
			.catch((err) =>
				expect(err.message).to.equal(`Returned error: VM Exception while processing transaction: revert.`)
			);
	});

	// Requires error handling
	it.skip("should fail for invalid parameters", async function () {
		const contract = new context.web3.eth.Contract(
			[{ ...TEST_CONTRACT_ABI[0], inputs: [{ internalType: "address", name: "a", type: "address" }] }],
			FIRST_CONTRACT_ADDRESS,
			{ from: GENESIS_ACCOUNT, gasPrice: "0x3B9ACA00" }
		);
		await contract.methods
			.multiply("0x0123456789012345678901234567890123456789")
			.call()
			.catch((err) =>
				expect(err.message).to.equal(`Returned error: VM Exception while processing transaction: revert.`)
			);
	});
});
