import { expect } from "chai";
import { step } from "mocha-steps";

import { createAndFinalizeBlock, describeWithFrontier } from "./util";

describeWithFrontier("Frontier RPC (Block)", `simple-specs.json`, (context) => {
	let previousBlock;
	// Those tests are dependant of each other in the given order.
	// The reason is to avoid having to restart the node each time
	// Running them individually will result in failure

	step("should be at block 0 at genesis", async function () {
		expect(await context.web3.eth.getBlockNumber()).to.equal(0);
	});

	it("should return genesis block", async function () {
		expect(await context.web3.eth.getBlockNumber()).to.equal(0);

		const block = await context.web3.eth.getBlock(0);
		expect(block).to.include({
			author: "0x0000000000000000000000000000000000000000",
			difficulty: "0",
			extraData: "0x0000000000000000000000000000000000000000000000000000000000000000",
			gasLimit: 0,
			gasUsed: 0,
			//hash: "0x14fe6f7c93597f79b901f8b5d7a84277a90915b8d355959b587e18de34f1dc17",
			logsBloom:
				"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
			miner: "0x0000000000000000000000000000000000000000",
			number: 0,
			//parentHash: "0x2cc74f91423ba20e9bb0b2c7d8924eacd14bc98aa1daad078f8844e529221cde",
			receiptsRoot: "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			size: 533,
			stateRoot: "0x0000000000000000000000000000000000000000000000000000000000000000",
			//timestamp: 1595012243836,
			totalDifficulty: null,
			//transactions: [],
			transactionsRoot: "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			//uncles: []
		});

		expect(block.transactions).to.be.a("array").empty;
		expect(block.uncles).to.be.a("array").empty;
		expect((block as any).sealFields).to.eql([
			"0x0000000000000000000000000000000000000000000000000000000000000000",
			"0x0000000000000000",
		]);
		expect(block.hash).to.be.a("string").lengthOf(66);
		expect(block.parentHash).to.be.a("string").lengthOf(66);
		expect(block.timestamp).to.be.a("number");
	});

	let firstBlockCreated = false;
	step("should be at block 1 after block production", async function () {
		this.timeout(15000);
		await createAndFinalizeBlock(context.web3);
		expect(await context.web3.eth.getBlockNumber()).to.equal(1);
		firstBlockCreated = true;
	});

	step("should have valid timestamp after block production", async function () {
		const block = await context.web3.eth.getBlock("latest");
		const last5Minutes = (Date.now() / 1000) - 300;
		const next5Minutes = (Date.now() / 1000) + 300;
		expect(block.timestamp).to.be.least(last5Minutes);
		expect(block.timestamp).to.be.below(next5Minutes);
	});

	step("retrieve block information", async function () {
		expect(firstBlockCreated).to.be.true;

		const block = await context.web3.eth.getBlock("latest");
		expect(block).to.include({
			author: "0x0000000000000000000000000000000000000000",
			difficulty: "0",
			extraData: "0x0000000000000000000000000000000000000000000000000000000000000000",
			gasLimit: 0,
			gasUsed: 0,
			//hash: "0x14fe6f7c93597f79b901f8b5d7a84277a90915b8d355959b587e18de34f1dc17",
			logsBloom:
				"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
			miner: "0x0000000000000000000000000000000000000000",
			number: 1,
			//parentHash: "0x04540257811b46d103d9896e7807040e7de5080e285841c5430d1a81588a0ce4",
			receiptsRoot: "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			size: 539,
			stateRoot: "0x0000000000000000000000000000000000000000000000000000000000000000",
			//timestamp: 1595012243836,
			totalDifficulty: null,
			//transactions: [],
			transactionsRoot: "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
			//uncles: []
		});
		previousBlock = block;

		expect(block.transactions).to.be.a("array").empty;
		expect(block.uncles).to.be.a("array").empty;
		expect((block as any).sealFields).to.eql([
			"0x0000000000000000000000000000000000000000000000000000000000000000",
			"0x0000000000000000",
		]);
		expect(block.hash).to.be.a("string").lengthOf(66);
		expect(block.parentHash).to.be.a("string").lengthOf(66);
		expect(block.timestamp).to.be.a("number");
	});

	step("get block by hash", async function() {
		const latest_block = await context.web3.eth.getBlock("latest");
		const block = await context.web3.eth.getBlock(latest_block.hash);
		expect(block.hash).to.be.eq(latest_block.hash);
	});

	step("get block by number", async function() {
		const block = await context.web3.eth.getBlock(1);
		expect(block).not.null;
	});

	it.skip("should include previous block hash as parent", async function () {
		this.timeout(15000);
		await createAndFinalizeBlock(context.web3);
		const block = await context.web3.eth.getBlock("latest");
		expect(block.hash).to.not.equal(previousBlock.hash);
		expect(block.parentHash).to.equal(previousBlock.hash);
	});
});
