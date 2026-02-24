
with open('simpleddns-app/src/main.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

for i, line in enumerate(lines[120:140]):
    print(f"{i+121}: {line.rstrip()}")
