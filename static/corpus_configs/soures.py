from glob import glob
import re
import os

paths = glob("./*.toml")
sources_re = re.compile(r"sources\s*=\s*\[\s*\"\.\/static\/(text\/[^\"]+)\"\s*\]")

for i, p in enumerate(paths):
    lang = p.split("/")[-1].split(".")[0]

    with open(p, "r", encoding="utf-8") as f:
        content = f.read()
        match sources_re.match(content):
            case None:
                continue
            case m:
                path = f'../../{m.group(1)}'
                print(path)
                print(os.path.isdir(path))
                replace = sources_re.sub(r'sources = [ "../../\1" ]', content)
                # if not content.endswith("\n"):
                #     content += "\n"        
        
                # replace = f'sources = [ "./static/text/{lang}" ]\n\n{content}'
                
                print(replace)
        
                with open(p, "w", encoding="utf-8") as f:
                    f.write(replace)

