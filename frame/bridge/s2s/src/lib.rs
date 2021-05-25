
pub struct Erc20Info {
    address: EthereumAddress,
    name: String,
    symbol: String,
    decimal: u8,
    value: u128,
};

pub enum Token {
    Native(u128),
    Erc20(Erc20Info),
};

pub struct RedeemInfo {
    token: Token,
    recipient: EthereumAddress,
};

