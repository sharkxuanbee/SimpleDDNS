
with open('simpleddns-app/src/main.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if 'struct DdnsApp {' in line:
        print(f"Struct start at {i+1}: {line.strip()}")
        # Print until we find impl or end of file
        for j in range(i+1, min(i+100, len(lines))):
            print(f"{j+1}: {lines[j].strip()}")
            if 'impl' in lines[j]:
                print("Found impl, stopping")
                break
        break
