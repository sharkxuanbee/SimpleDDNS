
with open('simpleddns-app/src/main.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

for i, line in enumerate(lines[160:230]):
    print(f"{i+161}: {line.rstrip()}")
