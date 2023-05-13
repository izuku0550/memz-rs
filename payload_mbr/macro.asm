%macro set_cursor 2
  mov ah, 02h
  mov bh, 0h
  mov dh , %1
  mov dl, %2
  int 10h
%endmacro

%macro sleep 2
  ; Use BIOS interrupt to sleep
  push ax ; push ah, al
  mov ah, 86h ; pause
  mov cx, %1
  mov dx, %2
  int 15h ; intrrupt
  pop ax ; restore ah, al
%endmacro

%macro clear 0
  mov ah, 06h ; scroll up the window
  mov al, 0 ; clear entire screen
  mov bh, 0h ; display attribute (white on black)
  mov ch, 0 ; row to start clearing from
  mov cl, 0 ; column to start clearing from
  mov dh, 24 ; row to end clearing at
  mov dl, 79 ; column to end clearing at
  int 10h ; call BIOS interrupt
%endmacro