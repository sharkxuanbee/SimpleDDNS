
import os

def format_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # heuristic formatting
        formatted = content.replace('; ', ';\n').replace(' { ', ' {\n').replace(' } ', ' }\n').replace('} ', '}\n').replace('/// ', '\n/// ')
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(formatted)
        print(f"Formatted {filepath}")
    except Exception as e:
        print(f"Error formatting {filepath}: {e}")

directory = 'simpleddns-core/src'
for filename in os.listdir(directory):
    if filename.endswith(".rs"):
        format_file(os.path.join(directory, filename))
