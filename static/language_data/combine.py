from os.path import dirname
import json

CURRENT_FOLDER = dirname(__file__)

def load_json(language: str, weight) -> dict:
    with open(f"{CURRENT_FOLDER}/{language}.json", 'r', encoding='utf8') as json_file:
        obj = json.load(json_file)
        for metric in obj.values():
            if type(metric) == dict:
                for ngram in metric.keys():
                    metric[ngram] *= weight

        return obj


def add_jsons(obj1: dict, obj2: dict, new_language: str) -> dict:
    obj1["language"] = new_language

    for metric in obj2.keys():
        if type(obj2[metric]) == dict:
            for ngram in obj2[metric].keys():
                try:
                    obj1[metric][ngram] += obj2[metric][ngram]
                except KeyError:
                    obj1[metric][ngram] = obj2[metric][ngram]
            # print(sorted(obj1[metric].items()))
            obj1[metric] = dict(sorted(obj1[metric].items(), key=lambda x: x[1], reverse=True))
    
    return obj1


def save_json(obj: dict):
    name = obj["language"]
    print(name)
    with open(f"{CURRENT_FOLDER}/{name}.json", 'w+', encoding='utf8') as new_file:
        json.dump(obj, new_file, indent='\t', separators=(',', ': '), ensure_ascii=False)


(lang1, weight1) = ("french_qu", 50)
(lang2, weight2) = ("english", 50)

weight_sum = weight1 + weight2
weight1 = weight1/weight_sum
weight2 = weight2/weight_sum

data1 = load_json(lang1, weight1)
data2 = load_json(lang2, weight2)

new_data = add_jsons(data1, data2, "frq-en_50-50")

save_json(new_data)