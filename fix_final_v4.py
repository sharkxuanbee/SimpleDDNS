
import os

filepath = 'simpleddns-app/src/main.rs'

if os.path.exists(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Line 27: for monospace fonts (Logs)
        if 'for monospace fonts (Logs)' in content:
             content = content.replace('for monospace fonts (Logs)', '// for monospace fonts (Logs)\n')
        
        # Line 105: for shutdown signal
        # The line might be `for shutdown signal` or `// Keep the main thread alive and wait for shutdown signal` split?
        # Previous script fixed `// Keep...` by adding newline?
        # If it added newline after `wait`, then `for shutdown signal` is code.
        if 'for shutdown signal' in content:
             content = content.replace('for shutdown signal', '// for shutdown signal\n')
             
        # Also check `for live status updates` if it wasn't fixed before
        if 'for live status updates' in content:
             content = content.replace('for live status updates', '// for live status updates\n')

        # Also `for each profile if missing`
        if 'for each profile if missing' in content:
             content = content.replace('for each profile if missing', '// for each profile if missing\n')

        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"Fixed final v4 issues in {filepath}")
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
