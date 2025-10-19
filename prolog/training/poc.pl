    % :- module(seq, [n_consecutifs/3]).

    n_consecutifs(N, Start, L) :-
        integer(N), N >= 0,
        integer(Start),
        End is Start + N - 1,
        range_(Start, End, L).

    range_(A, B, []) :- A > B, !.
    range_(A, B, [A|R]) :-
        A =< B,
        A1 is A + 1,
        range_(A1, B, R).