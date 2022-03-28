import { expect, assert } from "chai";
import Web3 from "web3";
import { config } from "./config";
import { customRequest } from "./utils";

let currentFilterId = null;

describe("Test filter API", function () {

	afterEach(async () => {
		if (currentFilterId) {
			const res = await customRequest("eth_uninstallFilter", [currentFilterId]);
		}
		currentFilterId = null;
	});

	it("should return a number as hexstring eth_newBlockFilter", async function () {
		const params = [];
		currentFilterId = await customRequest("eth_newBlockFilter", params);
		assert.isNumber(Web3.utils.hexToNumber(currentFilterId.result));
	});

	it.skip("should return a number as hexstring eth_newPendingTransactionFilter", async function () {
		const params = [];
		currentFilterId = await customRequest("eth_newPendingTransactionFilter", params);
		assert.isNumber(currentFilterId);
	});

	it("should return a number as hexstring when all options are passed with single address eth_newFilter", async function () {
		const params = [
			{
				fromBlock: "0x1", // 1
				toBlock: "0x2", // 2
				address: "0xfd9801e0aa27e54970936aa910a7186fdf5549bc",
				topics: ["0x317b31292193c2a4f561cc40a95ea0d97a2733f14af6d6d59522473e1f3ae65f", "0x0000000000000000000000006be02d1d3665660d22ff9624b7be0551ee1ac91b"],
			},
		];
		currentFilterId = await customRequest("eth_newFilter", params);
		assert.isNumber(Web3.utils.hexToNumber(currentFilterId.result));
	});

	it('should return a number as hexstring when all options are passed with address array', async function () {
		const params = [{
			"fromBlock": "0x1", // 1
			"toBlock": "0x2", // 2
			"address": ["0xfd9801e0aa27e54970936aa910a7186fdf5549bc", "0xab9801e0aa27e54970936aa910a7186fdf5549bc"],
			"topics": ["0x317b31292193c2a4f561cc40a95ea0d97a2733f14af6d6d59522473e1f3ae65f", "0x0000000000000000000000006be02d1d3665660d22ff9624b7be0551ee1ac91b"]
		}]
		currentFilterId = await customRequest('eth_newFilter', params);
		assert.isNumber(Web3.utils.hexToNumber(currentFilterId.result));
	});

	it('should return a number as hexstring when all options with "latest" and "pending" for to and fromBlock', async function () {
		const params = [{
			"fromBlock": "latest", // 1
			"toBlock": "pending", // 2
			"address": "0xfd9801e0aa27e54970936aa910a7186fdf5549bc",
			"topics": ["0x317b31292193c2a4f561cc40a95ea0d97a2733f14af6d6d59522473e1f3ae65f", "0x0000000000000000000000006be02d1d3665660d22ff9624b7be0551ee1ac91b"]
		}]
		currentFilterId = await customRequest('eth_newFilter', params);
		assert.isNumber(Web3.utils.hexToNumber(currentFilterId.result));
	});

	it('should return a number as hexstring when a few options are passed', async function () {
		const params = [{
			"fromBlock": "0x1", // 1
			"toBlock": "0x2", // 2
		}]
		currentFilterId = await customRequest('eth_newFilter', params);
		assert.isNumber(Web3.utils.hexToNumber(currentFilterId.result));
	});

	it('should return an error when no parameter is passed', async function () {
		const res = await customRequest('eth_newFilter', []);
		expect(res['error'].code, -32602);
	});

	it('should return an error when no parameter is passed', async function () {
		const res = await customRequest('eth_getFilterLogs', []);
		expect(res['error'].code, -32602);
	});

	it('should return an error when no parameter is passed', async function () {
		const res = await customRequest('eth_uninstallFilter', []);
		expect(res['error'].code, -32602);
	});

	it('should return a list of logs, when asking without defining an address and using toBlock "latest"', async function () {
		var params = [{
			"fromBlock": '0x0',
			"toBlock": 'latest'
		}]
		currentFilterId = await customRequest('eth_newFilter', params);
		const logs = await customRequest('eth_getFilterLogs', [currentFilterId.result]);
		assert.isArray(logs['result']);
	}).timeout(200000);
;

	it('should return a list of logs, when asking without defining an address and using toBlock "pending"', async function () {
		var params = [{
			"fromBlock": '0x0',
			"toBlock": 'pending'
		}]
		currentFilterId = await customRequest('eth_newFilter', params);
		const logs = await customRequest('eth_getFilterLogs', [currentFilterId.result]);
		assert.isArray(logs['result']);
	}).timeout(200000);

	it('should return a list of logs, when filtering with defining an address and using toBlock "latest"', async function () {
		expect(config.jsontestAddress).not.be.empty;
		var params = [{
			"address": config.jsontestAddress,
			"fromBlock": '0x0',
			"toBlock": 'latest'
		}]
		currentFilterId = await customRequest('eth_newFilter', params);
		const logs = await customRequest('eth_getFilterLogs', [currentFilterId.result]);
		assert.isArray(logs['result']);
	}).timeout(200000);

	it('should return a list of logs, when filtering with defining an address and using toBlock "pending"', async function () {
		expect(config.jsontestAddress).not.be.empty;
		var params = [{
			"address": config.jsontestAddress,
			"fromBlock": '0x0',
			"toBlock": 'pending'
		}]
		currentFilterId = await customRequest('eth_newFilter', params);
		const logs = await customRequest('eth_getFilterLogs', [currentFilterId.result]);
		assert.isArray(logs['result']);
	}).timeout(200000);

	it('should return a list of logs, when filtering by topic "0x0000000000000000000000000000000000000000000000000000000000000001"', async function () {
		var params = [{
			"fromBlock": '0x0',
			"toBlock": 'latest',
			"topics": ['0x0000000000000000000000000000000000000000000000000000000000000001']
		}]
		currentFilterId = await customRequest('eth_newFilter', params);
		const logs = await customRequest('eth_getFilterLogs', [currentFilterId.result]);
		assert.isArray(logs['result']);
	}).timeout(200000);

	it('should return a list of anonymous logs, when filtering by topic "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"', async function () {
		var params = [{
			"fromBlock": '0x0',
			"toBlock": 'latest',
			"topics": [null, null, '0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff']
		}]
		currentFilterId = await customRequest('eth_newFilter', params);
		const logs = await customRequest('eth_getFilterLogs', [currentFilterId.result]);
		assert.isArray(logs['result']);
	}).timeout(200000);

	it('should return a list of logs, when filtering by topic "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"', async function () {
		var params = [{
			"fromBlock": '0x0',
			"toBlock": 'latest',
			"topics": [null, null, null, '0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff']
		}]
		currentFilterId = await customRequest('eth_newFilter', params);
		const logs = await customRequest('eth_getFilterLogs', [currentFilterId.result]);
		assert.isArray(logs['result']);
	});
});
