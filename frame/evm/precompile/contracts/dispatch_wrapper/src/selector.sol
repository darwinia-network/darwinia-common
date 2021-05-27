// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.6.0;

interface ParachainStaking {
    //0x224fdd11
    function millau_backing_cross_receive(address,address) external view returns (bool);
    //0xa80b039a
    function pangolin_issuing_cross_receive(address,address) external view returns (bool);
}

