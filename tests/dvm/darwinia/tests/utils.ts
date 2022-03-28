import { assert } from "chai";
import Web3 from "web3";
import { config } from "./config";
import { BigNumber } from "bignumber.js";

var web3 = new Web3(config.host);

export function customRequest(method, params) {
	return new Promise((resolve, reject) => {
		(web3.currentProvider as any).send(
			{
				method: method,
				params: params,
				jsonrpc: "2.0",
				id: config.rpcMessageId++,
			},
			function (error, result) {
				if (error) {
					reject(
						`Failed to send custom request (${method} (${params.join(",")})): ${
							error.message || error.toString()
						}`
					);
				}
				resolve(result);
			}
		);
	});
}
