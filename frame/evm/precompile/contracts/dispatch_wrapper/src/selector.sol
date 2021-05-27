// SPDX-License-Identifier: GPL-3.0-only
pragma solidity >=0.6.0;

interface ParachainStaking {
    //0x3308e87a
    function millau2pangolin_backing_cross_receive(address,address) external view returns (bool);
}
