# chess_rust

## Objectives

This chess program is for educative purpose and a motivation to learn Rust and improve my AI skills. 

We don't want to be too specific to chess games. We will not go to far in the use of heuristics. For 
example, we will not use Syzygy tables for chess finals or we will not chess libraries. Our target is to:
- Get an engine that is not too strong and not to bad,
- Improve its level with the help of AI.

## Getting started

We currently use Arena Chess as our GUI to play with or run tournaments.

### GUI interface

Please follow these steps:
- Install Rust. When executed, the programs awaits for uci commands (see below)
- Install Arena Chess
- Build the code by typing the command: ```cargo build --release```
- Locate the executable under ```target/release/chess_rust```
- Add the engine in Arena Chess
- Play

### CLI

Or if you want to use the CLI, here is an example. Type the 2 commands one after the other:
 - ```./target/release/chess_rust```
- ```position startpos```
- ```go```.

## Algorithms

- Minimax (I have done negamax in fact)
- Alpha beta
- Mat solver (forced moves only)
- IDDFS

And there are heuristics:

- Transposition table: we avoid computing again an already evaluated move. The depth of analysis is memorized too (max_depth - current_depth),
- Preordering of moves (Transposition table, Killer moves, Capture, Check, ),
- Recapture if capture at the max depth,
- Aspiration window,
- Killer moves (moves causing a prune when: alpha(parent) >= beta (maximisation case) or alpha >= beta(parent) (minimisation case) ) 

The evaluation of a position is very basic. It is based on:
- Material evaluation
- Squares control (sum of controled squares for each piece)
- End games evaluation: a draw game is worth 0, a white mat is worth +∞ and a black mat is worth -∞. And we detect position with insufficient material like King and Knight agains King.
- To force a draw, we added a bonus / malus. 

### Forcing a draw

Let us consider the following position using the FEN notation: 

```8/7P/5k2/7K/8/3n4/8/8 b - - 4 80```

A naive evaluation of the Black position is worth approximatively +2 pawns. The Black player will never take the pawn since the new value of the position will worth 0. The reason is insuficient material for mat. Thus it will prefer to stay in the illusion of having an advantage. 

The correct move is ```80 Kg7```. And afterwards, black will play the wrong move: ```81 Kh8```

If we want Black to take the pawn by playing the move ```81 Kxh7```, we have to add a heuristic. We detect that Black cannot win. Thus we give a very important malus to the position to force the draw.

### Controlled squares

To compute controlled squares for both sides, we generate moves for Rook, Bishop, Knight, Queen and King. We just do not consider check.

For pawns, we generate attacked squares by using bitwise operations directly on the pawn bitboard.

## TODO

- Null move pruning

- Add randomness to avoid playing the same games several times (it is for tournament)

- Make the engine configurable
	- Set max_depth first
	- Add other parameters for making tuning easier (enable / disable some heuristics for example)

- Time management: this is a prerequiste to make tournaments against other computers. We do not want to lose a game just because of lack of time.

- Reuse generated moves for Rook, Bishop, Knight, Queen during the controlled squares computation:
	- To generate moves for one side more efficiently
	- To filter legal moves (a move is legal if the king of the current player is not anymore under attack after the move)

- Add ponderation (human against computer)
- Add parallelism
- Add statistics to check efficiency of preordering and pruning
- Evaluate the ELO strength in an industrial way ?
- Check if Aspiration Window really works. More generally, add traces in a file:
 . Tree variant with scores
 . Store alpha and beta pruning
 . Store preordering to check it is done correctly
- Develop a tool to analyse the logs

These traces and statistics will be useful for later improvements like use of NNUEs .

- Improve speed: by adding controlled squares stage and other consideration, we slowed down the computation by a factor of 100. 
    - We should reuse generated moves. More generally, there are likely to be several optimisations there by refactoring the code.
	- In this version, I think there are allocations that could be avoided
	- I should consider magic numbers (maybe I could enhance speed by a factor of 2).

## Next steps

Before going further, we need to be able to evaluate the strengh of our engine. As a minimum, we should be able to make it fight against another versions of itself:

### Preparation

- Step 1: Add time management
- Step 2: Make max_depth and heuristics configurable
- Step 3: Benchmark the engine on several positions
	- Use EPD (extended Position Description) test suites
		- manage EPD format (FEN + am, bm, id)
		- Compute a score based on the results:
			- check answer vs am / bm
			- Time to play			

	- Make tournaments between itself with different configurations and compute a score based on
		- Game result: win, draw, loss
		- Time to play for each side
		
- Step 4: Add AI.

### Add time management

We need to:
- Compute a time limit for a move during the IDDFS 
- Cancel the computation if it exceeds the time limit and if there is a best move available from the previous depth
	- Retrieve the last best move found
	- Cancel the alpha_beta engine

### Make the engine configurable

UCI has predefined commands to configure an engine. See ```setoption```.

### EPD Test suite automation

We can consider [python-chess](https://python-chess.readthedocs.io/en/latest/) to manage tests and results.

Look too for the GUI part at: [PyChess](https://pychess.github.io/about/) for tournament or continue using Arena Chess.

For the scoring part, we will define a time limit and then apply for each test the rules:
- The move played is in bm: bm_score = 1
- The move played is in am: am_score = 1
- Under the time limit: time_bonus += 1
Remark: check if there is an evaluation too.

Total score:
```Score = w1.bm_score + w2.(1 - am_score) + w3.time_bonus```
where ```w1 + w2 + w3 = 1``` with for example ```(w1, w2, w3) = (0.7, 0.2, 0.1)```.

And we sum all the scores to get the final evaluation.

### Addition of AI

As a first try we want to do something simple like a NNUE. 

Idea of features:
- Bitboard + position state (player turn, castle, capture en passant, ...): this is about encoding.
- Time management tuning
	- If a game is lost due to a bad time management, give a negative reward as a first approach. 

Consider actions call:
 	. Mat solver with <max_depth> variable,
	. Alphabeta with <alpha>, <beta> and <max_depth> variables
	. Aspiration window (can fail)
	- Position evaluation (stop simulating)
	- Mcts simulations
If for example we call too often mat solver whereas there is no threats, we will lose time and should give negative reward. How to give a reward on the opposite cases ? 

Gpu Based (like with Alphago) ?
- Score evaluation
- Move prediction

Avoid the use of convolutional network and consider only CPU based computing (NNUE) as a first approach. Analyse what we can do with a NNUE.


## Conclusion

We reached a first goal. The computer responds almost instantly but above a max depth of 3 moves (6 half moves), it starts to be slow. Going up to 4 moves seems currently difficult. Even if we optimize moves generation, we will hit a glass ceiling at a depth of 5.

The idea now is to focus on AI to fasten the moves selection and evaluation. And for that, we will prepare some benchmarks first.