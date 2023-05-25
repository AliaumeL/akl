function! AklCreate()
    "let s:rootdir = b:vimtex.root
    let document  = system("./akltex.py open  ".b:vimtex.root)
    call inputsave()
    let name = input('Enter destination name: ')
    call inputrestore()
    call inputsave()
    let url = input('Enter the full url: ')
    call inputrestore()
    let cmd =  './akltex.py create "' . document . '" "' . name . '" "' . url . '"'
    let text = system(cmd)
    execute "normal! i" . text . "\<Esc>"
endfunction

function! AklInsert()
    let document  = system("./akltex.py insert  ".b:vimtex.root)
    execute "normal! a" . document
endfunction

nnoremap <C-c> : call AklCreate()<CR>:w!<CR>
inoremap <C-c> <Esc>: call AklInsert()<CR>
