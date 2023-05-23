# Payload

## Kernel(Load MBR)

MEMZ의 페이로드는 MBR 영역(0x7C00)에 쓰이게 되며 메세지, PC 스피커, 이미지를 출력합니다.  

* 메세지 출력하기

MEMZ의 Payload에 있는 시작 메세지에 관한 코드를 만들기 전에 이 코드가 어떻게 작동하는지 알아야 합니다.  
우선 원본 페이로드 작동 방식은 아래와 같이 작동됩니다.  

<p align="center"><img src="md/gif/MEMZ Payload message.gif"></p>

글자색은 **검정색(0x0)** 이고 배경색은 **흰색(0xF)** 입니다.  
또한 글자마다 시간에 간격을 두고 출력되고 있다는 것을 실행을 통해 알 수 있었습니다.
그리고 이 문자열은 `compressed.bin(음악, 이미지, 텍스트 압축 파일)`에 저장되어 있습니다.  
현재로서는 `compressed.bin`을 활용하는 코드를 이해하지 못했으므로 따로 `text` 파일만 추출하여 테스트를 합니다.  
제가 지금 결과만 보고 알 수 있는 정보는 이게 끝이었고 원본 kernel source code를 확인해보니 아래 코드가 쓰였습니다.  

```asm
mov cx, 0xb800 ; Set base address for video memory
mov es, cx
```

위 코드는 [Text mode에서 VGA를 사용](https://en.wikipedia.org/wiki/VGA_text_mode)하기 위해 비디오 메모리를 지정하는 코드였고 그 뒤에 나오는 `startmsg` 레이블에 있는 코드 중

```asm
sleep 0x0, 0x6000

cmp si, image+24000+476+msglen
jge note

lodsb
mov ah, 0xf0
stosw
```

위 코드가 메세지를 출력하는 코드인 것 같아 명령어를 하나하나 분석해보니

* sleep: 주어진 인수만큼 시스템을 대기시키는 매크로

```asm
...
mov ah, 86h ; 시스템을 주어진 시간만큼 대기시키는 인터럽트
...
int 15h
...
```

* `cmp, jge` 코드는 `image+24000+476+msglen`의 의미(아마도 메세지 주소 관련인 것 같습니다)를 정확하게 분석하질 못해서 지금은 알 수 없어서 패스..
* 아래 코드가 결정적인 출력을 담당하는 코드인데

```asm
lodsb
mov ah, 0xf0
stosw
```

`lodsb`는 `ds:si`에 저장된 메모리 주소의 바이트 값을 `al` 레지스터에 로드하고 `si`의 포인터를 증가시키는 명령어이다. 이 명령어를 통해 문자열을 쉽게 출력하는 것이 가능하고 문자에 배경색을 주기 위해 ah 레지스터에 `0xf(white, background)`를 추가하여 stosw로 `al(문자)`과 `ah(색상정보)`를 합쳐서 저장합니다.  

원리를 알았으니 제가 직접 만든 코드를 보겠습니다.

```asm
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
  mov di, 0       ; di 레지스터의 값을 0으로 설정

  clear           ; 화면 초기화

  mov si, msg ; si가 msg의 시작 주소 저장
  mov cx, msglen ; cx(count register)에 msglen 저장

start_message:
  .loop:
    lodsb                   ; si가 현재 가리키는 byte를 al에 저장하고 si의 포인터를 1 증가시킴
    cmp al, 0
    je .done                ; al의 값이 0이 되면 종료
    mov byte[es:di], al     ; es 레지스터에서 di가 가리키는 위치에 문자(al) 값을 저장
    inc di                  
    mov byte[es:di], 0xF0   ; es 레지스터에서 di가 가리키는 위치에 색상정보를 저장
    inc di
    sleep 0x0, 0x6000       ; 0x6000 만큼 대기
    jmp .loop               ; 반복
  .done:
    hlt                     ; 종료(프로세서의 동작을 멈춤)

msg: incbin "data/other/text.bin", 0 ; text.bin에 있는 데이터를 포함시키기
msglen: equ $ - msg ; msg 레이블과 현재 위치($)의 거리를 계산하여 길이를 계산

times 510 - ($-$$) db 0 ; 510 Byte 중 남은 영역을 0으로 채우기
dw 0xAA55 ; 2 Byte는 Boot Sector 서명으로 사용
```

* 이미지 출력하기

[ X ]

* PC 스피커 제어하기

[ X ]

## 참고

* [INT 10H](https://ko.wikipedia.org/wiki/INT_10H)
* [부트로더 화면 제어](https://eclipsemode.tistory.com/16)
* [내 PC를 부팅하자](https://github.com/HIPERCUBE/64bit-Multicore-OS/blob/master/book/Ch4_%EB%82%B4%20PC%EB%A5%BC%20%EB%B6%80%ED%8C%85%ED%95%98%EC%9E%90.md)
* [MEMZ](https://github.com/NyDubh3/MEMZ)
