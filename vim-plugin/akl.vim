function! AklPaste()
    "let s:rootdir = b:vimtex.root
    let text  = system("./akltex.py create ".b:vimtex.root)
    execute "normal! i" . text . "\<Esc>"
endfunction

inoremap <C-c> <Esc>: call AklPaste()<CR>
