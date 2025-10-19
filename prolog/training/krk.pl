    % generator 0..63 without library(between)
    num(A, B, A) :- A =< B.
    num(A, B, X) :- A < B, A1 is A + 1, num(A1, B, X).

    square(S) :- num(0, 63, S).

    sq_file(S, F) :- F is S mod 8.
    sq_rank(S, R) :- R is S // 8.

    % kings are adjacent if Chebyshev distance is 1
    kings_adjacent(S1, S2) :-
        sq_file(S1, F1), sq_rank(S1, R1),
        sq_file(S2, F2), sq_rank(S2, R2),
        DF is abs(F1 - F2),
        DR is abs(R1 - R2),
        max3(DF, DR, M),
        M =:= 1.

    % a simple max/3
    max3(A,B,M) :- A >= B, M is A.
    max3(A,B,M) :- A <  B, M is B.

    % KRK positions: distinct squares, non-adjacent kings
    krk(WK, WR, BK) :-
        square(WK),
        square(WR), WR =\= WK,
        square(BK), BK =\= WK, BK =\= WR,
        \+ kings_adjacent(WK, BK).