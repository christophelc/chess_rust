% scryer-prolog -g run_tests krk_strategy.pl krk_strategy_test.pl

% krk_strategy_test.pl  (no directives needed)
:- use_module(library(between)).
:- use_module(library(lists), [member/2, length/2]).

ok(Name)     :- write('ok - '), write(Name), nl.
nok(Name, E) :- write('not ok - '), write(Name), write(' '), write(E), nl.

must_succeed(G) :- ( call(G) -> true ; throw(error(test_failed(G),_)) ).
must_fail(G)    :- ( call(G) -> throw(error(expected_failure(G),_)) ; true ).
must_equal(X,Y) :- ( X == Y -> true ; throw(error(unexpected_result(X,Y),_)) ).

run_one(Name, Goal) :-
    (   catch(
            ( call(Goal),
              !,
              ok(Name)
            ),
            E,
            ( write('not ok - '), write(Name), write(' '), write(E), nl,
              fail
            )
        )
    ->  true
    ;   % Si le test a simplement échoué, on essaie quand même d'afficher le résultat principal
        ( catch(show_result(Name), _, true),
          write('not ok - '), write(Name), write(' failed'), nl )
    ).

% Helper pour afficher la stratégie si disponible
show_result :-
    ( krk_strategy(WK, WR, BK, From, Move),
      write('=> chosen: '), write(WK-WR-BK-From-Move), nl,
      fail
    ; true ).

show(Label, X) :- write(Label), write(': '), write(X), nl.

run_tests :-
    write('--- KRK strategy tests ---'), nl,
    run_one('manhattan a1-a8 = 7', (manhattan(0,56,D), must_equal(D,7))),
    run_one('king_adjacent e4-d5',  king_adjacent(28,35)),
    run_one('rook_line a1-a8 ok',   rook_line(0,56,[])),
    run_one('rook_line a1-a8 blocked at a4',
            must_fail(rook_line(0,56,[24]))),
    % ---- krk_normalize_square/2 --------------------------------------------
    % ---- tests de normalisation (attention: virgules, pas de points) ----
    run_one('krk_normalize 15 -> 15',
        ( krk_normalize_square(15, N), must_equal(N, 15) )),
    run_one('krk_normalize move(rook,15) -> 15',
        ( krk_normalize_square(move(rook, 15), N), must_equal(N, 15) )),
    run_one('krk_normalize move(king,0) -> 0',
        ( krk_normalize_square(move(king, 0), N), must_equal(N, 0) )),
    run_one('krk_normalize negative -> -7',
        ( krk_normalize_square(move(any, -7), N), must_equal(N, -7) )),
    run_one('krk_normalize foo fails',
        must_fail(krk_normalize_square(foo, _))),
    run_one('krk_normalize move(rook,x) fails',
        must_fail(krk_normalize_square(move(rook, x), _))),
    run_one('krk_normalize move/3 fails',
        must_fail(krk_normalize_square(move(rook, 15, extra), _))),
    run_one('strategy (WK=c2,WR=b1,BK=a8) returns a legal move',
    ( WK=10, WR=1, BK=56,     % c2, b1, a8 — rook not directly aligned with BK
        krk_strategy(WK,WR,BK, From, Move),
        ( Move = move(rook, WR_to) ->
            % rook move must be a legal slide without crossing WK or BK
            rook_line(WR, WR_to, [WK,BK]),
            WR_to =\= WR, WR_to =\= BK
        ; Move = move(king, WK_to) ->
            king_step_towards(WK, BK, WK_step),
            WK_to = WK_step,
            legal_wk_square(WK_to, WR, BK),
            \+ rook_attacks(WK_to, WR, [BK])
        )
    )),
    run_one('strategy (WK=60,WR=63,BK=4) chooses rook to h2 (15)', 
    ( WK=60, WR=63, BK=4,
        krk_strategy(WK,WR,BK, From, Move),
        must_equal(From, WR),
        must_equal(Move, move(rook, 15)),
        % Sanity: glissade de tour légale et sûre
        rook_line(WR, 15, [WK,BK]),
        15 =\= WR, 15 =\= WK, 15 =\= BK,
        king_cannot_capture_rook(WK, 15, BK)
    )),
    write('All tests finished.'), nl.
