# [A MVP Design of a Relayer Fee Market]

## Requirements

1. Users use native tokens of the source chain as the only method of payment;
2. The estimated fee of the message to be transferred is supported by a pricing system on the source chain which is connected to a Relayer Fee Market;
3. The cost on the target chain is at the expense of the relayer who claims the handling fee on the source chain with the proof of delivery after delivery is completed successfully. To incentivize relayers, the pricing system should ensure the gain is greater than the cost. If an automatic pricing mechanism is infeasible, relayers should be able to give their offers manually and shoulder the cost of offering. The relayers should be pushed if they fail to relay the message as expected.
4. Relayers and Users constitute a secondary supply-and-demand market, where prices rise when supply is low and fall when supply is abundant. There are no access restrictions for relayers, and anyone can enter. Relayer should evaluate and quote at their own discretion as an economically rational person. An incomplete list of risks that relayers should take into account is as follows:
    1. Fluctuation in token prices and exchange ratios;
    2. Time delay between quoting and claiming;
    3. Loss of staking funds due to software or network failures

## Proposal A—Three-tiered Quotation

This approach is suitable for scenarios with lower gas fee on the source chain and shorter finality time. It has better versatility, reliability, and robustness. Such networks include Heco, BSC, Polygon and Darwinia.

First, the relayer posts their quotes based on the reference price and the expected profit on the blockchain at any time. An off-chain pricing system maintains the reference price. Each relayer should lock a sufficient default margin on the chain to guarantee the faithful execution of the deal.

In this way, a series of ***Ask*** prices come into being in the ascending order on the blockchain. When the user initiates a request on the source chain, the three lowest offers **_P1<P2<P3_** are filtered out and $P3$  is used as the billing price.  The user can request a cross-chain message delivery after paying **_P3_**.

Then the system requires the three relayers with respect to **_P1_**, **_P2_**, and **_P3_** to execute the delivery during three consecutive time slots: **_0~T1_**,  **_T1~T2_**, and **_T2~T3_** respectively, where **_T1_**, **_T2_**, and **_T3_** can be configured suitable for the scenario. If the relayer fails to deliver in their allotted time slot, the task goes to the next relayer. The penalty fee **_S(t)_** from the previous relayer also adds up to the gain of the current relayer as a bonus, where **_S(t)_** is a function of the delay **_t_**.

### Detailed Steps in Implementation

1. A relayer first registers and deposits some funds (tokens) as the collateral if they are willing to participate.
    1. enroll_and_lock_collateral dispatch call; relayer enrollment mapping <id, collateral_amount>
    2. cancel_enrollment() dispatch call (and return collateral, require checkin existing orders)
2. ***Ask***, asks(price) dispatch call,  updates ask list storage (sorted) 
    1. Internal call for querying current price(return **_P1_**, **_P2_**, **_P3_** and related relayers)
    2. RPC
    3. Update, Cancel
3. Internal call for bid and execution, some user pay **_P3_** for sending a cross-chain message.
    1. Create a new order(with current time), in the order lifecycle, relayer cannot cancel enrollment and take back collateral.
    2. **_P3_** is locked in the Order(contract). 
    3. Managing orders storage
4. Internal call for finishing an order or slash an order after message delivery.
    1. Finish order if this message is delivered by required relayers in the order, and pay **_P3_**.
    2. Slash order if the message is delivered by other relayers, pay **_P3 + S(t)_** to relayer who delivered message, and slash S(t) from required relayers(of **_P1, P2, P3_**) collateral.
    3. How to prove this message is delivered by specific relayer?
        1. When some relayer deliver message, it will attach an extra data describing the source account of relayer.
        2. The source account will be included in the MessageDelivery event in target chain.
        3. Thus source account will be included in the Message Delivery Proof.
    4. Can other relayer help claim with the message delivery proof? And will the protocol incentive this behavior?
5. If the collateral of any relayer is lower than required collateral threshold (e.g. slashed in order), the enrolment of this relayer will become inactive(will be removed from the ***ask*** list, and not able to put new ***ask***).
6. Distribution of system revenue
    1. In future, the protocol need to capture some income from the fees, so we might need to set a ratio **_R_** (similar to tax) from the fees. **_(P3 + S(t)) * (1-R)_** will go to relayer, others will go to treasury for now. (To be determined: Who pays for it? How much?
    2. Payment to the relayer can be broken down further (header relay, message relay, message delivery proof) *To be implemented in the future*
7. Time Line:
    1. Solution 1(One problematic case is that **_P2_**'s relayer may forestall P1's relayer to complete the delivery):
        1. During **_0 ~ T1_**, only **_P1_**'s relayer can participate. If **_P1_**'s relayer succeed delivery
        2. During **_T1 ~ T2_**,  only **_P1_** and **_P2_**.   If **_P1_** or **_P2_** 's relayer succeed delivery, pay relayer's ask price, will not slash **_P1_**'s relayer, if the delivery relayer is **_P2_**'s.
        3. During **_T2 ~ T3_**, only **_P1, P2, P3_**, will not slash **_P1_** and **_P2_**'s relayer, if the delivery relayer is **_P3_**'s, other cases are similar.
        4. **_T3~_** , any relayer.
    2. Solution 2 (Any relayer can also do the same thing as Solution 1):
        1. **_0 ~ T, P1, P2, P3_** are all legible to participate，pay relayer's ask price(or P3).
    3. Solution 3 (P2 takes P1's Header Relay):
        1. **_0 ~ T, P1, P2, P3_** are all legible to participate in the ***reply*** process. Suppose ***S*** is the source_account of the one who completes the message delivery on the target chain and the source_account is none of **_P1_**, **_P2_**, or **_P3_**, then any one of **_P1_**, **_P2_**, or **_P3_** can claim the gain with the proof of delivery. If the source_account is one of **_P1_**, **_P2_**, or **_P3_**, only they can claim the gain.
        2. **_T~_**, Only relayer delivers and anyone can reply (The gain is distributed between the relayer and the replier)

    4. Solution 4 (Selected):
        1. **_0 ~ T1, P1,_**  Suppose ***S*** is the source_account of the one who completes the message delivery on the target chain and the source_account is not P1, then P1 can claim the gain with the proof of delivery. pay relayer's ask price，If the source_account is P1, only they can claim the gain.  
        2. **_T1 ~ T2, P2,_**  Suppose ***S*** is the source_account of the one who completes the message delivery on the target chain and the source_account is not P2, then P1 can claim the gain with the proof of delivery. pay relayer's ask price，If the source_account is P2, only they can claim the gain.  
        3. **_T2 ~ T3, P3,_** Suppose ***S*** is the source_account of the one who completes the message delivery on the target chain and the source_account is not P3, then P1 can claim the gain with the proof of delivery. pay relayer's ask price，If the source_account is P3, only they can claim the gain. 
        4. **_T3~_**, Only relayer delivers and anyone can reply (The gain is distributed between the relayer and the replier)
    5. Solution 5

        Solution 4 + Header Relay  Provides proof of delivery and storage query. The relayer of Header Relay is also included in the message delivered.

    Example:

    lock_and_remote_issue

    1. user lock, ***locked_asset***
    2. send issue_from_remote cross-chain message
    3. Bid and create delivery order
    4. relayer delivery
        1. relayer sync header
        2. sync message
            1. message delivered in target chain
            2. message call execute on remote chain (success/failure), e.g. issuing a mapping token.
            3. Deposit MessageDeilvery(,...... message_execute_result) event
    5.  if message_execute_result is success, move ***locked_asset*** to backing vault. else, return the ***locked_asset*** back to user.

## Proposal B—Oracle+On-chain Automatic Pricing

High gas fees in some networks, such as Ethereum, may prevent the relayer from quoting frequently, and the execution cost of message delivery on the target chain is predictable, for example (***Ethereum>Darwinia***). In this scenario, a second-best solution is to query the execution cost by the interface on the target chain, plus the estimated delivery cost. The disadvantage is that it is not adaptable, and it is possible that no relayer is willing to take the order, causing message delivery congestion and stability problems. 

## Update to Darwinia > Ethereum Bridge: Grandpa Beefy Light Client + Three-tiered Quotation

For BEEFY, the interaction is a multi-round process in which BridgedChain fee should be paid. The user needs to know in advance how much the handling fee is and whether the amount is sufficient. However, it can not be predicted. We can establish a market which implements a set of ***ask***/***bid*** system.

The relayer posts a quote for ***Header Relay*** during a specific period(***ask***) and the user may respond to it (***bid***) if they accept the quoted price. The relayer relays the **header** after the deal is closed. The relayer may lose the staking tokens if they fail to relay the message in time, whatever the reason is. More than one relayer can quote at the same time to compete for users.