
import os

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        
        # We need to find `struct DdnsApp {` and the subsequent `impl`
        # and ensure `}` is between them.
        
        struct_start = -1
        impl_start = -1
        
        for i, line in enumerate(lines):
            if 'struct DdnsApp {' in line:
                struct_start = i
            if struct_start != -1 and 'impl' in line and 'DdnsApp' in line:
                impl_start = i
                break
        
        if struct_start != -1 and impl_start != -1:
            # Check if there is a } between struct_start and impl_start
            has_close = False
            for k in range(struct_start, impl_start):
                if '}' in lines[k]:
                    has_close = True
                    break
            
            if not has_close:
                # Insert } before impl_start
                # But we should insert it nicely.
                # lines[impl_start] starts with `impl ...`
                # We insert `}\n` before it.
                
                lines.insert(impl_start, '}\n\n')
                
                with open(filepath, 'w', encoding='utf-8') as f:
                    f.writelines(lines)
                print(f"Fixed missing struct closing brace in {filepath}")
            else:
                print("Struct seems to be closed")
        else:
            print("Could not find struct or impl")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
