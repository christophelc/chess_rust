# chess_rust

## Objectives

This chess program is for educative purpose and a motivation to learn Rust.

## Algorithms

- Minimax (I have done negamax in fact)
- Alpha beta
- Mat solver (forced moves only)
- IDDFS

And there are heuristics:

- Preordering of moves
- Recapture if capture at the max depth
- Aspiration window

## TODO

- Enrich the position evaluation ? It is probably the cause we cannot prune too much since there is currently no way to discriminate different position except by considering captured material. We will try to train a neural networt to evaluate the position as done with AlphaGo To be analysed.

- Add statistics to check preordering and pruning efficiency
- Check if Aspiration Window really works. More generally, add traces in a file:
 . tree variant with scores
 . store alpha and beta pruning
 . store preordering to check it is done correctly
- Develop a tool to analyse the logs

These traces and statistics will be useful for later improvements like use of NNUEs .

- Time management
- Refactoring of the code
- Improve speed (not a priority)
	- In this version, I think there are allocation that could be avoided
	- I should consider magic number (may be I could enhance speed by a factor of 2).

## Next steps

As a first try we want to do something simple like a NNUE. 

Idea of features:
- Bitboard + position state (player turn, castle, capture en passant, ...): this is about encoding.
- Time management: 
 - if a game is lost due to a bad time management, give a negative reward as a first approach. 
 - Consider action call:
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
