// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.6.0;

interface ParachainStaking {
    //0x6774148c
    function s2sissuing_cross_send(address token, address recipient, uint256 amount) external view returns (bool);
}
