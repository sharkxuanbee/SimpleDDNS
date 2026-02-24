
with open('simpleddns-core/src/resolver.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Simple formatting
formatted = content.replace('; ', ';\n').replace(' { ', ' {\n').replace(' } ', ' }\n').replace('} ', '}\n')

with open('simpleddns-core/src/resolver.rs', 'w', encoding='utf-8') as f:
    f.write(formatted)
