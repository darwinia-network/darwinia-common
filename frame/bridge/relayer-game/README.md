## Steps Into MMR Proof

- **Target Chain: Ethereum**
- **Sampling Strategy: One By One**
- **Relay Block's Number: 4**
- **Game Id: 4**
- **Last Confirmed Block's Number on Darwinia When Game(4) Started: 1**
- **The Proof Order is From Foot to Top, then Left to Right**

---

1. **Round 0, Target 4**
	```
	Global:      A      | Current:      A
	           /   \    |             /   \
	          c     f   |            c     f
	         / \   / \  |           / \   / \
	        a   b d   e |          a   b d   e
	        1   2 3   4 |          1   2 3   4

	This Proposal Say: I think the MMR Root is A, and I prove it contains a(block 1's MMR Hash)
	Proof: [b, f]
	    A
	   / \
	  -   f
	 / \
	a   b
	```

1. **Round 1, Sample 3**
	```
	Global:      A      | Current:     B
	           /   \    |             / \
	          c     f   |            c   \
	         / \   / \  |           / \   \
	        a   b d   e |          a   b   d
	        1   2 3   4 |          1   2   3

	This Extended Prove: A(previous MMR Root) contains d(block 3's MMR Hash)
	Proof: [e, c]
	  A
	 / \
	c   -
	   / \
	  d   e
	```

1. **Round 2, Sample 2**
	```
	Global:      A      | Current:
	           /   \    |
	          c     f   |            C
	         / \   / \  |           / \
	        a   b d   e |          a   b
	        1   2 3   4 |          1   2

	This Extended Prove: B(previous MMR Root) contains b(block 2's MMR Hash)
	Proof: [a, d]
	    B
	   / \
	  -   \
	 / \   \
	a   b   d
	```

1. **Reach Last Confirmed Block, Game Over**
