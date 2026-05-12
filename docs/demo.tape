Output docs/demo.gif

Set Shell "bash"
Set FontSize 14
Set Width 1200
Set Height 700
Set Theme "Dracula"
Set Framerate 24
Set PlaybackSpeed 1.5

# launch githut
Type "githut"
Enter
Sleep 2s

# search for neovim
Type "/"
Type "neovim"
Sleep 200ms
Enter
Sleep 3s

# navigate results, README auto-loads
Down
Sleep 400ms
Down
Sleep 400ms
Down
Sleep 1.5s

# open file browser
Type "l"
Sleep 2s

# navigate into a dir
Down
Sleep 300ms
Down
Sleep 300ms
Type "l"
Sleep 1.5s

# go back up
Type "h"
Sleep 500ms
Type "h"
Sleep 500ms

# save a single file
Down
Sleep 300ms
Type "c"
Sleep 500ms
Type "~/demo-file.lua"
Sleep 200ms
Enter
Sleep 1.5s

# back to results
Escape
Sleep 500ms

# sparse clone
Type "C"
Sleep 500ms
Type "~/sparse-demo"
Sleep 200ms
Enter
Sleep 500ms
Type "runtime/lua"
Sleep 200ms
Enter
Sleep 4s

# open issues
Type "i"
Sleep 2s

# navigate issues
Down
Sleep 400ms
Down
Sleep 400ms
Type "l"
Sleep 1.5s

# back
Escape
Sleep 500ms
Escape
Sleep 500ms

# notifications
Type "3"
Sleep 1.5s

# quit
Type "q"
Sleep 500ms
