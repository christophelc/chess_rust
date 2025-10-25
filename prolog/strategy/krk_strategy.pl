%%%% --- Board geometry helpers (0-based) --------------------------------------

:- use_module(library(between)).
:- use_module(library(lists), [member/2, length/2]).

file(S, F) :- F is S mod 8.
rank(S, R) :- R is S // 8.
sq(F, R, S) :- S is R*8 + F.
square(S) :- S >= 0, S =< 63.

manhattan(S1, S2, D) :-
  file(S1,F1), rank(S1,R1),
  file(S2,F2), rank(S2,R2),
  DX is abs(F1-F2), DY is abs(R1-R2),
  D is DX + DY.

king_adjacent(S1, S2) :-
  file(S1,F1), rank(S1,R1),
  file(S2,F2), rank(S2,R2),
  abs(F1-F2) =< 1, abs(R1-R2) =< 1,
  S1 =\= S2.

king_neighbour(S1, S2) :-
  file(S1, F1), rank(S1, R1),
  between(-1, 1, DF), between(-1, 1, DR),
  (DF =:= 0, DR =:= 0 -> fail ; true),
  F2 is F1 + DF, R2 is R1 + DR,
  F2 >= 0, F2 =< 7, R2 >= 0, R2 =< 7,
  sq(F2, R2, S2).


edge_square(S) :-
  file(S,F), rank(S,R),
  (F=:=0; F=:=7; R=:=0; R=:=7).

step_towards(A, B, Out) :-      % one-step Chebyshev component
  ( A < B -> Out is A+1
  ; A > B -> Out is A-1
  ;            Out = A ).

king_step_towards(Src, Target, Dst) :-
  file(Src,Fs), rank(Src,Rs),
  file(Target,Ft), rank(Target,Rt),
  step_towards(Fs,Ft,Fd),
  step_towards(Rs,Rt,Rd),
  sq(Fd,Rd,Dst),
  square(Dst).

path_clear_same_file(From, To, Blockers) :-
  file(From,F), file(To,F), rank(From,R1), rank(To,R2),
  MinR is min(R1,R2) + 1, MaxR is max(R1,R2) - 1,
  \+ ( MinR =< MaxR,
       between(MinR, MaxR, Rb),
       sq(F,Rb,B), member(B, Blockers) ).

path_clear_same_rank(From, To, Blockers) :-
  rank(From,R), rank(To,R), file(From,F1), file(To,F2),
  MinF is min(F1,F2) + 1, MaxF is max(F1,F2) - 1,
  \+ ( MinF =< MaxF,
       between(MinF, MaxF, Fb),
       sq(Fb,R,B), member(B, Blockers) ).

rook_line(From, To, Blockers) :-
  ( file(From,F), file(To,F), path_clear_same_file(From,To,Blockers)
  ; rank(From,R), rank(To,R), path_clear_same_rank(From,To,Blockers)
  ).

rook_attacks(From, Target, Blockers) :-
  rook_line(From, Target, Blockers).

% king_attacks(K, S) :- king_adjacent(K, S).
king_attacks(K, S) :- king_neighbour(K, S).

occupied(S, WK, WR, BK) :- member(S, [WK,WR,BK]).

%%%% --- Position legality checks ---------------------------------------------

legal_wk_square(WKN, WR, BK) :-
  square(WKN),
  WKN =\= WR, WKN =\= BK,
  \+ king_adjacent(WKN, BK).

king_cannot_capture_rook(WK, WRN, BK) :-
  ( \+ king_adjacent(BK, WRN)
  ; king_attacks(WK, WRN) ).

%%%% --- Black king legal moves (after White moves) ---------------------------

bk_legal_move_exists(WK, WR, BK, BK2) :-
  king_attacks(BK, BK2),
  square(BK2),
  BK2 =\= WK, BK2 =\= WR,
  \+ king_adjacent(BK2, WK),
  \+ rook_attacks(WR, BK2, [WK]).

bk_has_legal_move(WK, WR, BK) :-
  bk_legal_move_exists(WK, WR, BK, _).

%%%% --- Check detection (after White moves) ----------------------------------

bk_in_check(WK, WR, BK) :-
  rook_attacks(WR, BK, [WK]).

%%%% --- Helpers to enumerate all rook targets on WR's rank/file ---------------

rook_targets_on_rank_file(WR, BK, Blockers, WR_to) :-
  % any square sharing file with WR or rank with WR, excluding WR itself
  ( file(WR,F), between(0,7,R), sq(F,R,WR_to)
  ; rank(WR,Rk), between(0,7,F2), sq(F2,Rk,WR_to)
  ),
  WR_to =\= WR,
  WR_to =\= BK,
  \+ member(WR_to, Blockers),
  rook_line(WR, WR_to, Blockers).

%%%% --- 1) Try a mating rook move --------------------------------------------

try_mate(WK, WR, BK, move(rook, WR_to)) :-
  rook_targets_on_rank_file(WR, BK, [WK,BK], WR_to),
  king_cannot_capture_rook(WK, WR_to, BK),
  bk_in_check(WK, WR_to, BK),
  \+ bk_has_legal_move(WK, WR_to, BK).

%%%% --- 2) Try a space-shrinking rook move (safe, non-hanging) ---------------

try_rook_shrink(WK, WR, BK, move(rook, Best)) :-
  setof(Score-Cand,
        ( rook_targets_on_rank_file(WR, BK, [WK,BK], Cand),
          king_cannot_capture_rook(WK, Cand, BK),
          mobility(BK, WK, Cand, Score)
        ),
        Pairs),
  Pairs = [ _-Best | _ ].

% Mobility = number of BK legal moves after the rook move (lower is better)
mobility(BK, WK, WR_to, Score) :-
  findall(BK2, bk_legal_move_exists(WK, WR_to, BK, BK2), Moves),
  length(Moves, Score).

%%%% --- 3) King progress toward BK (avoid blocking rook) ---------------------

try_king_progress(WK, WR, BK, move(king, WK_to)) :-
  king_step_towards(WK, BK, WK_cand),
  legal_wk_square(WK_cand, WR, BK),
  % avoid stepping onto the rook line segment between WR and BK
  \+ rook_attacks(WK_cand, WR, [BK]),
  WK_to = WK_cand.

% Normalisation robuste, sans exceptions : accepte N ou move(_, N)
krk_normalize_square(move(_, M), N) :-
    catch(N is M + 0, _, fail), !.
krk_normalize_square(M, N) :-
    catch(N is M + 0, _, fail), !.

krk_strategy(WK, WR, BK, MoveFrom, MoveTo) :-
  ( try_mate(WK, WR, BK, M) ->
        krk_normalize_square(M, MoveTo), MoveFrom is WR
  ; try_rook_shrink(WK, WR, BK, M) ->
        krk_normalize_square(M, MoveTo), MoveFrom is WR
  ; try_king_progress(WK, WR, BK, M) ->
        krk_normalize_square(M, MoveTo), MoveFrom is WK
  ).
