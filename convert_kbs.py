from glob import glob
from pathlib import Path
import json

for dirs in glob("./**/*.kb", recursive=True):
    with open(dirs, "r", encoding="utf-8") as file:
        content = file.read()
        
    name = dirs.split("/")[-1].split(".")[0]
    
    rows = [s.strip() for s in content.split("\n")]
    
    if len(rows) > 3 and rows[3] != '':
        print(name, len(rows), dirs, "\n", rows)
        continue
        
    if len(rows) < 3:
        print(name, len(rows), dirs, "\n", rows)
        continue
    
    dirs = Path(f"test-dofs/{"/".join(dirs.split("/")[:-1])}")
    dirs.mkdir(parents=True, exist_ok=True)
    
    path = f"{dirs}/{name}.dof"
    
    dof = {
       	"name": name,
    	"board": "ortho",
    	"layers": {
    		"main": [
    			rows[0],
    			rows[1],
    			rows[2]
    		]
    	},
    	"fingering": "traditional"
    }
    
    with open(path, "w+", encoding='utf-8') as file:
        file.write(json.dumps(dof, indent=4, ensure_ascii=False))
    
