
with open('simpleddns-app/src/main.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if 'match self' in line:
        print(f"Line {i+1}: {line.strip()}")
        for j in range(1, 10):
            if i+j < len(lines):
                print(f"Line {i+1+j}: {lines[i+j].strip()}")
