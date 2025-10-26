def is_between(a: Int32, b: Int32, x: Int32): Bool = a <= x and x <= b

def is_square(a: Int32): Bool = is_between(0, 63, a)

def is_king_adjacent(wk: Int32, bk: Int32): Bool =
    let f1 = wk % 8;
    let r1 = wk / 8;
    let f2 = bk % 8;
    let r2 = bk / 8;
    let df = Int32.abs(f1 - f2);
    let dr = Int32.abs(r1 - r2);
    let m  = if df >= dr then df else dr;
    m == 1

def main(): Unit \ IO = {
    // On déclare explicitement le schéma des prédicats :
    let rules: #{
        Square(Int32),
        KingsAdjacent(Int32, Int32),
        KRK(Int32, Int32, Int32)
    } = #{
        // Génère toutes les cases 0..63 puis filtre avec is_square (inutile ici mais didactique)
        Square(s) :- s = 0..63.

        // Génère toutes les paires de cases et garde celles où les rois seraient adjacents
        KingsAdjacent(s1, s2) :-
            s1 = 0..63,
            s2 = 0..63,
            is_king_adjacent(s1, s2).

        // Positions KRK valides (très simplifié) :
        KRK(wk, wr, bk) :-
            Square(wk),
            Square(wr), wr != wk,
            Square(bk), bk != wk, bk != wr,
            not KingsAdjacent(wk, bk).
    };

    let results = query rules select (wk, wr, bk) from KRK(wk, wr, bk);
    results |> List.take(5) |> println
}

/*
mod Krk {
    // Équivalent "case class" en Flix
    struct Pos[r](wk: Int32, wr: Int32, bk: Int32)

    // Programme logique typé : pas de `rel`, on liste les prédicats du set.
    def program(): #{
      Num(Int32, Int32, Int32),
      Square(Int32),
      KingsAdjacent(Int32, Int32),
      KRK(Pos)
    } =
      #{
        // Générateur 0..63
        Num(a, b, a) :- a <= b.
        Num(a, b, x) :- a < b, a1 = a + 1, Num(a1, b, x).

        // Cases 0..63
        Square(s) :- Num(0, 63, s).

        // Helpers purs utilisables dans les règles
        // (définis en dehors du set, appelés dans les guards)
        // Chebyshev(s1,s2) = max(|df|,|dr|) == 1  => rois adjacents
        KingsAdjacent(s1, s2) :-
          Square(s1), Square(s2),
          f1 = sqFile(s1), r1 = sqRank(s1),
          f2 = sqFile(s2), r2 = sqRank(s2),
          df = absInt(f1 - f2),
          dr = absInt(r1 - r2),
          m  = max2(df, dr),
          m == 1.

        // Positions KRK: cases distinctes, rois non adjacents
        KRK(Pos(wk, wr, bk)) :-
          Square(wk),
          Square(wr), wr != wk,
          Square(bk), bk != wk, bk != wr,
          not KingsAdjacent(wk, bk).
      };

    // Petites fonctions pures (utilisées dans les règles ci-dessus)
    def absInt(x: Int32): Int32 = if (x >= 0) x else -x
    def max2(a: Int32, b: Int32): Int32 = if (a >= b) a else b
    def sqFile(s: Int32): Int32 = s % 8
    def sqRank(s: Int32): Int32 = s / 8

    // Utilitaire: récupérer quelques positions
    pub def examples(n: Int32): List[Pos] & Pure =
        let p = program();
        query p select pos from KRK(pos) |> List.take(n)
}

// Point d’entrée
def main(): Unit \ IO =
    let xs = Krk.examples(5);
    println(xs)
*/