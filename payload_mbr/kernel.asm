use16
org 0x00

section .text       ; text 섹션(세그먼트) 정의
jmp 0x07C0:start    ; cs 세그먼트 레지스터에 0x07C0을 복사하면서 start 레이블로 이동

%include "macro.asm"

start:
  mov ax, cs
  mov ds, ax      ; ds 세그먼트 레지스터에 설정
  
  mov ax, 0xB800  ; 비디오 메모리의 시작 어드레스(0xB800)를 세그먼트 레지스터 값으로 변환 
  mov es, ax      ; es 레지스터에 설정
  mov di, 0

  clear

  mov si, msg ; si가 msg의 시작 주소 저장
  mov cx, msglen ; cx(count register)에 msglen 저장

start_message:
  .loop:
    lodsb ; si가 현재 가리키는 byte를 al에 저장하고 si의 포인터를 1 증가시킴
    cmp al, 0
    je .done ; al의 값이 0이 되면 종료
    mov byte[es:di], al ; 
    inc di
    mov byte[es:di], 0xF0
    inc di
    sleep 0x0, 0x6000
    jmp .loop
  .done:
    hlt

msg: incbin "data/other/text.bin", 0
msglen: equ $ - msg

times 510 - ($-$$) db 0
dw 0xAA55