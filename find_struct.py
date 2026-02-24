
with open('simpleddns-app/src/main.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if 'struct ProfileEditor' in line:
        print(f"Struct ProfileEditor at {i+1}: {line.strip()}")
        for j in range(1, 15):
            print(f"{i+1+j}: {lines[i+j].rstrip()}")
        break
