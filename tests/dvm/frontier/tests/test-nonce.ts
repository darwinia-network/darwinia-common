import { expect } from "chai";
import { step } from "mocha-steps";

import { createAndFinalizeBlock, describeWithFrontier, customRequest } from "./util";

describeWithFrontier("Frontier RPC (Nonce)", (context) => {
	const GENESIS_ACCOUNT = "0x6be02d1d3665660d22ff9624b7be0551ee1ac91b";
	const GENESIS_ACCOUNT_PRIVATE_KEY = "0x99B3C12287537E38C90A9219D4CB074A89A16E9CDB20BF85728EBD97C343E342";
	const TEST_ACCOUNT = "0x1111111111111111111111111111111111111111";

	step("get nonce", async function () {
		this.timeout(10_000);
		const tx = await context.web3.eth.accounts.signTransaction({
			from: GENESIS_ACCOUNT,
			to: TEST_ACCOUNT,
			value: "0x200", // Must me higher than ExistentialDeposit (500)
			gasPrice: "0x3B9ACA00",
			gas: "0x100000",
		}, GENESIS_ACCOUNT_PRIVATE_KEY);

		expect(await context.web3.eth.getTransactionCount(GENESIS_ACCOUNT, 'earliest')).to.eq(0);

		await customRequest(context.web3, "eth_sendRawTransaction", [tx.rawTransaction]);

		expect(await context.web3.eth.getTransactionCount(GENESIS_ACCOUNT, 'latest')).to.eq(0);
		expect(await context.web3.eth.getTransactionCount(GENESIS_ACCOUNT, 'pending')).to.eq(1);

		await createAndFinalizeBlock(context.web3);

		expect(await context.web3.eth.getTransactionCount(GENESIS_ACCOUNT, 'latest')).to.eq(1);
		expect(await context.web3.eth.getTransactionCount(GENESIS_ACCOUNT, 'pending')).to.eq(1);
		expect(await context.web3.eth.getTransactionCount(GENESIS_ACCOUNT, 'earliest')).to.eq(0);
	});
});
