from itertools import permutations
from os import path
import pathlib
import json
from glob import glob
from random import choice


def path_to_language(path: str):
    return path.split("\\")[-1].split(".")[0]


def load_json(path: str):
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)


def doubles_for_lang(language: str):
    bigrams = load_json(f"static/language_data/{language}.json")['bigrams']
    res = 0
    for b, f in bigrams.items():
        if b[0] == b[1]:
            res += f
    return res * 100


def check_doubles(path: str):
    l = path_to_language(path)
    print(f"{l:-<16}: {doubles_for_lang(l)}%")


def filterPossibilities(p: str):
    c1, c2 = p[0], p[1]
    if c1 == "a":
        return c2 == "e"
    elif c1 == "e":
        return c2 == "a" or c2 == "i"
    elif c1 == "i":
        return c2 != "i"
    elif c1 == "o":
        return c2 == "i" or c2 == "u"
    else:
        return c2 == "a"


class Solution:
    def countVowelPermutation(self, n: int) -> int:
        pos = "aeiou"
        perms = (p for p in permutations(pos, n) if filterPossibilities(p))
        length = 0
        for _ in perms:
            length += 1
        return length % (10**9 + 7)


def main():
    print("doubles for each lanugage is:")
    # for p in glob("static/language_data/*.json"):
    #     check_doubles(p)
    e450k = open('450k.txt', 'r', encoding='utf8').read().split()
    res = [choice(e450k) for _ in range(100_000_000)]
    open('450k_corpus.txt', 'w+', encoding='utf8').write(" ".join(res))
    # print(path.isdir("../generator_release"))


if __name__ == "__main__":
    for p in glob("static/language_data/*.json")[14:]:
        name = path.basename(p).removesuffix(".json")

        with open(f"corpora/provided/{name}.toml", 'w+', encoding='utf8') as f:
            f.write("based_on = [\"default\"]")

        
    # s = Solution()
    # s.countVowelPermutation(3)