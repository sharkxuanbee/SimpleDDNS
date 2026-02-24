
import os

def format_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # heuristic formatting
        # Add newlines after semicolons, braces
        formatted = content.replace('; ', ';\n').replace(' { ', ' {\n').replace(' } ', ' }\n').replace('} ', '}\n')
        # Handle doc comments: ensure they start on a new line
        formatted = formatted.replace('/// ', '\n/// ')
        # fix potential double newlines
        formatted = formatted.replace('\n\n', '\n')
        
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(formatted)
        print(f"Formatted {filepath}")
    except Exception as e:
        print(f"Error formatting {filepath}: {e}")

directories = [
    'simpleddns-core/src',
    'simpleddns-storage/src',
    'simpleddns-providers/src',
    'simpleddns-app/src'
]

for directory in directories:
    if os.path.exists(directory):
        for filename in os.listdir(directory):
            if filename.endswith(".rs"):
                format_file(os.path.join(directory, filename))
