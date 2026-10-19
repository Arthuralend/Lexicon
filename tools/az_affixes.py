#!/usr/bin/env python3
"""Generate assets/spelling/az_AZ.aff, the affix rules of Lexicon's Azerbaijani dictionary.

The word list (az_AZ.dic, from mozillaz/spellchecker, MPL-2.0) tags each stem with its part of
speech and the harmony class of its last vowel (README of that project):

    N/O noun, A/J adjective, U/M numeral, P/R pronoun, D/B adverb, V/E verb, F/G conjunction
    1 = a ı (back unrounded), 2 = e ə i (front unrounded), 3 = o u (back rounded), 4 = ö ü (front rounded)

Its own .aff described only four of those classes (N1, O2, V1, E2) and few forms, so most words
were accepted only bare. This script writes rules for every class from a description of
Azerbaijani morphology: vowel harmony (A = a/ə, H = ı/i/u/ü, Q = q/k), buffer consonants after a
vowel ((n), (y), (s)), q→ğ and k→y before a vowel, the noun's plural, possessive, case and
predicative endings, and the verb's tenses, persons, negation, participles and converbs. Each
rule carries a whole ending, because Lexicon's affix engine chains at most two suffixes.

TRY, KEY, REP and MAP come from mozillaz/spellchecker's az.aff; the output stays MPL-2.0.

Usage: python3 tools/az_affixes.py > assets/spelling/az_AZ.aff
"""

VOWELS = "aıoueəiöü"
COND_V = "[aıoueəiöü]"

HARMONY = {"a": 1, "ı": 1, "e": 2, "ə": 2, "i": 2, "o": 3, "u": 3, "ö": 4, "ü": 4}


def back(h):
    return h in (1, 3)


def realize(morphemes, h, ends_vowel):
    """The surface form of `morphemes` after a stem of harmony class `h` ending in a vowel or not."""
    out = ""
    for m in morphemes:
        first = True
        i = 0
        while i < len(m):
            if m.startswith("(", i):
                j = m.index(")", i)
                inner = m[i + 1 : j]
                i = j + 1
                # (H) only after a consonant; (n), (y), (s) only after a vowel.
                if (inner == "H") == ends_vowel:
                    continue
                c = inner
            else:
                c = m[i]
                i += 1
            if c == "A":
                c = "a" if back(h) else "ə"
            elif c == "H":
                c = "ıiuü"[h - 1]
            elif c == "Q":
                c = "q" if back(h) else "k"
            if c in VOWELS:
                # A suffix starting with a vowel softens the q or k it follows (yazacaq → yazacağam).
                if first and out.endswith("q"):
                    out = out[:-1] + "ğ"
                elif first and out.endswith("k"):
                    out = out[:-1] + "y"
                h = HARMONY[c]
                ends_vowel = True
            else:
                ends_vowel = False
            out += c
            first = False
    return out


# Nouns -----------------------------------------------------------------------------------------

POSS = ["(H)m", "(H)n", "(H)mHz", "(H)nHz"]
POSS3 = ["(s)H", "lArH"]
CASE = ["", "(n)Hn", "(y)A", "(n)H", "dA", "dAn", "(y)lA"]
CASE3 = ["", "nHn", "nA", "nH", "ndA", "ndAn", "(y)lA"]
PERSONAL = ["(y)Am", "sAn", "(y)HQ", "sHnHz"]


def noun_endings():
    seqs = []
    for pl in ([], ["lAr"]):
        for case in CASE:
            seqs.append(pl + [case])
        for p in POSS:
            for case in CASE:
                seqs.append(pl + [p, case])
        for p in POSS3:
            if pl and p == "lArH":
                continue
            for case in CASE3:
                seqs.append(pl + [p, case])
        # Predicates: kitabdır, evdədir, evdəyəm, müəlliməm, kitablarıdır.
        for case in ["", "dA", "dAn", "(n)Hn"]:
            seqs.append(pl + [case, "dHr"])
        for case in ["", "dA"]:
            for pers in PERSONAL:
                seqs.append(pl + [case, pers])
        seqs.append(pl + ["(s)H", "dHr"])
        # evdəki, evdəkilər
        seqs.append(pl + ["dA", "kH"])
        seqs.append(pl + ["dA", "kH", "lAr"])
        seqs.append(pl + ["(n)Hn", "kH"])
    # Derived words with the commonest endings: kitablı, susuz, dostluq, işçi, türkcə.
    for d in ["lH", "sHz", "lHQ", "çH", "cA", "cAsHnA", "lAş"]:
        seqs.append([d])
        for pl in ([], ["lAr"]):
            for case in CASE:
                seqs.append([d] + pl + [case])
        seqs.append([d, "dHr"])
        seqs.append([d, "(s)H"])
    return seqs


# Verbs -----------------------------------------------------------------------------------------

PERS_A = ["(y)Am", "sAn", "", "(y)HQ", "sHnHz", "lAr"]
PERS_B = ["m", "n", "", "Q", "nHz", "lAr"]


def verb_endings():
    seqs = []

    def with_pers(stem, persons, extra=()):
        for p in persons:
            seqs.append(stem + [p])
        for e in extra:
            seqs.append(stem + [e])

    for neg in ([], ["mA"]):
        # Present: yazıram, yazmıram (the negative uses -mHr, not -mA + -Hr).
        pres = ["mHr"] if neg else ["(y)Hr"]
        with_pers(pres, PERS_A, ["dHr"])
        with_pers(pres + ["dH"], PERS_B)  # yazırdım
        with_pers(pres + ["mHş"], PERS_A)  # yazırmışam
        with_pers(neg + ["dH"], PERS_B)  # yazdım, yazmadım
        with_pers(neg + ["mHş"], PERS_A, ["dHr"])  # yazmışam
        with_pers(neg + ["mHs"], ["An", "HnHz"])  # yazmısan, yazmısınız
        with_pers(neg + ["(y)Hb"], ["", "lAr", "dHr", "sAn", "sHnHz"])  # yazıb, yazmayıb
        with_pers(neg + ["(y)AcAQ"], PERS_A, ["dHr", "dH"])  # yazacağam, yazmayacaq
        with_pers(neg + ["(y)AcAQ", "dH"], PERS_B)
        with_pers(neg + ["sA"], PERS_B)  # yazsam, yazmasam
        with_pers(neg + ["(y)A"], PERS_A)  # yazam, yazasan (optative)
        with_pers(neg + ["mAlH"], PERS_A, ["dHr"])  # yazmalıyam
        for imp in ["sHn", "(y)Hn", "(y)HnHz", "sHnlAr", "(y)AQ", "(y)Hm"]:
            seqs.append(neg + [imp])
        # Participles and converbs.
        for p in ["(y)An", "(y)AcAQ"]:
            for pl in ([], ["lAr"]):
                for case in CASE:
                    seqs.append(neg + [p] + pl + [case])
            seqs.append(neg + [p, "dHr"])
        for poss in POSS:
            for case in CASE:
                seqs.append(neg + ["dHQ", poss, case])
                seqs.append(neg + ["(y)AcAQ", poss, case])
        for poss in POSS3:
            for case in CASE3:
                seqs.append(neg + ["dHQ", poss, case])
                seqs.append(neg + ["(y)AcAQ", poss, case])
        for c in [["(y)ArAQ"], ["dHQ", "dA"], ["dHQ", "cA"], ["(y)HncA"], ["(y)AlH"], ["dHQ", "dAn"], ["mAdAn"]]:
            seqs.append(neg + c)
    # Aorist: yazaram, yazmaram, yazmazsan.
    with_pers(["(y)Ar"], PERS_A, ["dHr", "dH"])
    with_pers(["(y)Ar", "dH"], PERS_B)
    with_pers(["mAr"], ["(y)Am", "(y)HQ"])
    with_pers(["mAz"], ["sAn", "", "sHnHz", "lAr", "dH"])
    # Infinitive and verbal noun: yazmaq, yazmağa, yazmaqda, yazması.
    for case in ["", "(y)A", "(n)H", "dA", "dAn", "(n)Hn", "dHr", "lA", "dAnsA", "cHn"]:
        seqs.append(["mAQ", case])
    for pl in ([], ["lAr"]):
        for case in CASE:
            seqs.append(["mA"] + pl + [case])
    for case in CASE3:
        seqs.append(["mA", "(s)H", case])
    return seqs


# Rules -----------------------------------------------------------------------------------------


def rules_for(flag, h, seqs, nominal):
    """SFX lines for a class: each ending after consonant- and vowel-final stems."""
    out = []
    seen = set()
    alt = "q" if back(h) else "k"
    soft = "ğ" if back(h) else "y"

    def add(strip, affix, cond):
        if not affix and not strip:
            return
        key = (strip, affix, cond)
        if key not in seen:
            seen.add(key)
            out.append(f"SFX {flag} {strip or '0'} {affix or '0'} {cond}")

    for seq in seqs:
        after_c = realize(seq, h, False)
        after_v = realize(seq, h, True)
        if after_c == after_v and (not after_c or after_c[0] not in VOWELS):
            add("", after_c, ".")
            continue
        add("", after_v, COND_V)
        if after_c and after_c[0] in VOWELS and nominal:
            # Nouns: a final q or k softens before a vowel (otaq → otağı, çörək → çörəyi).
            add("", after_c, f"[^aıoueəiöü{alt}]")
            add(alt, soft + after_c, alt)
        else:
            add("", after_c, "[^aıoueəiöü]")
            if after_c and after_c[0] in VOWELS and not nominal and h == 2:
                # get, et: t becomes d before a vowel (gedirik, edirəm).
                add("t", "d" + after_c, "et")
    return out


HEADER = """# Azerbaijani affix rules for Lexicon, generated by tools/az_affixes.py (MPL-2.0).
# TRY, KEY, REP and MAP: mozillaz/spellchecker az.aff (MPL-2.0). Word list: az_AZ.dic.
SET UTF-8
TRY aliərQTntımdAsNqkyOouşzebücvçYpVhxfEgöğSBDjRUPİKLHƏFŞZCXGMÇĞIJÖÜ'
KEY qüertyuiopöğ|asdfghjklıə|zxcvbnmçş
LANG az_AZ
FLAG long
REP 2
REP o a
REP a o
MAP 7
MAP uü
MAP oö
MAP gğ
MAP iı
MAP eə
MAP cç
MAP sş
"""


def main():
    nouns = noun_endings()
    verbs = verb_endings()
    blocks = []
    for h in (1, 2, 3, 4):
        noun_flag = ("N" if back(h) else "O") + str(h)
        adj_flag = ("A" if back(h) else "J") + str(h)
        num_flag = ("U" if back(h) else "M") + str(h)
        pron_flag = ("P" if back(h) else "R") + str(h)
        adv_flag = ("D" if back(h) else "B") + str(h)
        verb_flag = ("V" if back(h) else "E") + str(h)
        conj_flag = ("F" if back(h) else "G") + str(h)
        blocks.append(rules_for(noun_flag, h, nouns, True))
        blocks.append(rules_for(adj_flag, h, nouns + [["rAQ"], ["rAQ", "dHr"]], True))
        blocks.append(rules_for(num_flag, h, nouns + [["(H)ncH"], ["(H)ncH", "dHr"], ["(H)ncH", "(s)H"]], True))
        blocks.append(rules_for(pron_flag, h, nouns, True))
        blocks.append(rules_for(adv_flag, h, [["dHr"], ["dA"], ["kH"]], True))
        blocks.append(rules_for(verb_flag, h, verbs, False))
        blocks.append(rules_for(conj_flag, h, [["(s)H"], ["dHr"]], True))
    print(HEADER, end="")
    for rules in blocks:
        if not rules:
            continue
        flag = rules[0].split()[1]
        print(f"\nSFX {flag} Y {len(rules)}")
        print("\n".join(rules))


if __name__ == "__main__":
    main()
