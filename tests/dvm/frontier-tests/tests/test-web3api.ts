import { expect } from "chai";
import { step } from "mocha-steps";

import { describeWithFrontier, customRequest } from "./util";

describeWithFrontier("Frontier RPC (Web3Api)", (context) => {
	step("should remote sha3", async function () {
		const data = context.web3.utils.stringToHex("hello");
		const hash = await customRequest(context.web3, "web3_sha3", [data]);
		const local_hash = context.web3.utils.sha3("hello");
		expect(hash.result).to.be.equal(local_hash);
	});
});
