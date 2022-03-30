import { expect } from "chai";

import { describeWithFrontier } from "./util";

// All test for the RPC

describeWithFrontier("Frontier RPC (Constant)", (context) => {
	it("should have 0 hashrate", async function () {
		expect(await context.web3.eth.getHashrate()).to.equal(0);
	});

	it("should have chainId 42", async function () {
		// The chainId is defined by the Substrate Chain Id, default to 42
		expect(await context.web3.eth.getChainId()).to.equal(42);
	});

	it("should have no account", async function () {
		expect(await context.web3.eth.getAccounts()).to.eql([]);
	});

	it("block author should be 0x0000000000000000000000000000000000000000", async function () {
		// This address `0x1234567890` is hardcoded into the runtime find_author
		// as we are running manual sealing consensus.
		expect(await context.web3.eth.getCoinbase()).to.equal(
			"0x0000000000000000000000000000000000000000"
		);
	});

	it("should gas price", async function () {
		expect(await context.web3.eth.getGasPrice()).to.equal("1000000000");
	});

	it("should protocal version is 1", async function () {
		expect(await context.web3.eth.getProtocolVersion()).to.equal(1);
	});

	it("should is syncing is false", async function () {
		expect(await context.web3.eth.isSyncing()).to.be.false;
	});
});
