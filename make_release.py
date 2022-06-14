from os import path
import json
from glob import iglob


def path_to_language(path: str):
    return path.split("\\")[-1].split(".")[0]


def load_json(path: str):
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)


def check_doubles(language: str):
    bigrams = load_json(f"static/language_data/{language}.json")['bigrams']
    res = 0
    for b, f in bigrams.items():
        if b[0] == b[1]:
            res += f
    return res * 100


def main():
    print("doubles for each lanugage is:")
    for l in iglob("static/language_data/*.json"):
        l = path_to_language(l)
        print(f"{l:-<16}: {check_doubles(l)}%")
    # print(path.isdir("../generator_release"))


if __name__ == "__main__":
    main()