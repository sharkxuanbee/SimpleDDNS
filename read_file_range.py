
with open('simpleddns-app/src/main.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

for i, line in enumerate(lines[290:500]):
    print(f"{i+291}: {line.rstrip()}")
