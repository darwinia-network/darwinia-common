## Steps Into MMR Proof

- **Target Chain: Ethereum**
- **Relay Block's Number: 5**
- **Game Id: 5**
- **Last Confirmed Block's Number on Darwinia When Game(5) Started: 1**

---

1. **Round 0, Target 5**
	```
	Root: A
	Global:      A      | Current:      A
	           /   \    |             /   \
	          3     6   |            3     6
	         / \   / \  |           / \   / \
	        1   2 4   5 |          1   2 4   5

	Proof: [1, 2, 6]
	    -
	   / \
	  -   6
	 / \
	1   2
	```

2. **Round 1, Sample 4**
	```
	Root: B
	Global:      A      | Current:     B
	           /   \    |             / \
	          3     6   |            3   \
	         / \   / \  |           / \   \
	        1   2 4   5 |          1   2   4

	Proof: [3, 4, 5]
	  A
	 / \
	3   -
	   / \
	  4   5
	```

3. **Round 2, Sample 3**
	```
	Root: C
	Global:      A      | Current:   C
	           /   \    |           / \
	          3     6   |          1   2
	         / \   / \  |
	        1   2 4   5 |

	Proof: [1, 4]
	    B
	   / \
	  -   \
	 / \   \
	1   -   4
	```
