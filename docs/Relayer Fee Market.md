# A MVP Design of a Relayer Fee Market

## Requirements

1. Users use native tokens of the source chain as the only method of payment;
2. The estimated fee of the message to be transferred is supported by a pricing system on the source chain which is connected to a Relayer Fee Market;
3. The cost on the target chain is at the expense of the relayer who claims the handling fee on the source chain with the proof of delivery after delivery is completed successfully. To incentivize relayers, the pricing system should ensure the gain is greater than the cost. If an automatic pricing mechanism is infeasible, relayers should be able to give their offers manually and shoulder the cost of offering. The relayers should be pushed if they fail to relay the message as expected.
4. Relayers and Users constitute a secondary supply-and-demand market, where prices rise when supply is low and fall when supply is abundant. There are no access restrictions for relayers, and anyone can enter. Relayer should evaluate and quote at their own discretion as an economically rational person. An incomplete list of risks that relayers should take into account is as follows:
    1. Fluctuation in token prices and exchange ratios;
    2. Time delay between quoting and claiming;
    3. Loss of staking funds due to software or network failures

## Proposal A-Three-tiered Quotation

This approach is suitable for scenarios with lower gas fee on the source chain and shorter finality time. It has better versatility, reliability, and robustness. Such networks include Heco, BSC, Polygon and Darwinia.

First, the relayer posts their quotes based on the reference price and the expected profit on the blockchain at any time. An off-chain pricing system maintains the reference price. Each relayer should lock a sufficient default margin on the chain to guarantee the faithful execution of the deal.

In this way, a series of ***Ask*** prices come into being in the ascending order on the blockchain. When the user initiates a request on the source chain, the three lowest offers **_P1<P2<P3_** are filtered out and **P3**  is used as the billing price.  The user can request a cross-chain message delivery after paying **_P3_**. The reason that we select 3 relayers in the executed order is that we want to have redundancy for executing the message delivery. P1's relayer is R1, P2's relayer is R2, P3's relayer is R3.

In any time, the message relayer and confirm relayer can be anyone, do not have be the assigned relayer. But in priority time slots, assigned relayer will be rewarded with much more regardless wo relay the message and do the confirmation.
To give them different priority for **R1, R2, R3**, we will split the T into three consecutive time slots: **_0~T1 for P1_**,  **_T1~T2 for P2_**, and **_T2~T3 for P3_** respectively. The relayer who is assigned with her own time slot will be rewarded with more percentage, and this reward is for the asked price and the commitment(delivery in time or slash). Relayer with lower price are assigned with earlier time slot.
### Detailed Steps in Implementation

1. Enroll and lock collateral
    1. `enroll_and_lock_collateral` dispatch call
    2. `cancel_enrollment()` dispatch call, remember to check if the relayer is in priority time slots.
2. Ask price
    1. Query **_P1_**, **_P2_**, **_P3_** and **_R1_**, **_R2_**, **_R3_**
    2. Update, Cancel prices storage
    3. If the collateral of any registered relayer is lower than required collateral threshold (e.g. slashed), the enrolment of this relayer will become inactive(will be removed from the ask list, and not able to put new ask).
3. Send message, user pay **_P3_** for sending a cross-chain message.
    1. Create a new order(with current block number), in the order lifecycle, relayer cannot cancel enrollment and take back collateral.
    2. **_P3_** is locked in the module relayer fund account.
4. Message delivery and confirmed by bridger.
5. Reward and Slash Strategy.
    1. **_0 ~ T1, assigned relayer: R1,_** R1 can claim 60% from the reward P1, and message relayer can claim 80% * (1 - 60%) from P1， confirm relayer can claim 20% * (1 - 60%) from P1, (P3 - P1) will go to treasury.
    2. **_T1 ~ T2, assigned relayer: R2,_** R2 can claim 60% from the reward P1, and message relayer can claim 80% * (1 - 60%) from P1， confirm relayer can claim 20% * (1 - 60%) from P1, (P3 - P1) will go to treasury.
    3. **_T2 ~ T3, assigned relayer: R3,_** R3 can claim 60% from the reward P1, and message relayer can claim 80% * (1 - 60%) from P1， confirm relayer can claim 20% * (1 - 60%) from P1, (P3 - P1) will go to treasury.
    4. **_T3~_**, The reward will be S(t) where S(t) > P3, the part S(t) - P3 comes from funds slashed from R1, R2, R3's collateral. Message relayer can claim 80% from S(t)， confirm relayer can claim 20% from S(t).

   Note: The ratio parameters in the strategy can be defined in runtime, and there might be update to them for refinement after more benchmark and statistics.
## Proposal B-Oracle + On-chain Automatic Pricing

High gas fees in some networks, such as Ethereum, may prevent the relayer from quoting frequently, and the execution cost of message delivery on the target chain is predictable, such as (***Ethereum>Darwinia***). In this scenario, a second-best solution is to query the execution cost by the interface on the target chain, plus the estimated delivery cost. The disadvantage is that it is not adaptable, and it is possible that no relayer is willing to take the order, causing message delivery congestion and stability problems.

## Update to Darwinia > Ethereum Bridge: Grandpa Beefy Light Client + Three-tiered Quotation

For BEEFY, the interaction is a multi-round process in which BridgedChain fee should be paid. The user needs to know in advance how much the handling fee is and whether the amount is sufficient. However, it can not be predicted. We can establish a market which implements a set of ***ask***/***bid*** system.

The relayer posts a quote for ***Header Relay*** during a specific period(***ask***) and the user may respond to it (***bid***) if they accept the quoted price. The relayer relays the **header** after the deal is closed. The relayer may lose the staking tokens if they fail to relay the message in time, whatever the reason is. More than one relayer can quote at the same time to compete for users.
