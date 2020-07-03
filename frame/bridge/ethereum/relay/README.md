## Steps Into MMR Proof

- **relay: 5**
- **game id: 5**
- **last confirmed on chain when game(5) started: 1**

---

1. **round 0, target 5**
	```
	root: A
	     A
	   /   \
	  3     6
	 / \   / \
	1   2 4   5

	proof: [(1), 2, 6]
	    -
	   / \
	  -   6
	 / \
	1   2
	```

2. **round 1, sample 4**
	```
	root: B
	    B
	   / \
	  3   \
	 / \   \
	1   2   4

	proof: [3, (4), 5]
	  A
	 / \
	3   -
	   / \
	  4   5
	```

3. **round 2, sample 3**
	```
	root: C
	  C
	 / \
	1   2

	proof: [1, 4]
	    B
	   / \
	  3   \
	 / \   \
	1   2   4
	```
