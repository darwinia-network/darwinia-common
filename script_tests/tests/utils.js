
const Web3 = require('web3');
const web3 = new Web3('http://localhost:9933');

function customRequest(method, params) {
    return new Promise((resolve, reject) => {
        web3.currentProvider.send({
            method: method,
            params: params,
            jsonrpc: "2.0",
            id: 1
        }, function (error, result) {
            if (error) {
                reject(
                    `Failed to send custom request (${method} (${params.join(",")})): ${
                        error.message || error.toString()
                    }`
                );
            }
            resolve(result)
        })
    })
}

exports.customRequest = customRequest;