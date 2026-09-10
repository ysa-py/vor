package sshchain

import (
	"bufio"
	"context"
	"crypto/rand"
	"crypto/sha1"
	"encoding/base64"
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/url"
	"strings"
	"time"
)

// wsDial performs an RFC 6455 client handshake and returns a net.Conn that
// transparently frames outgoing bytes as masked binary frames and unpacks
// incoming frames (the SSH stream never sees the framing).
//
// transport is an already-established stream (e.g. a TLS conn for wss://);
// when nil, addr is dialed as plain TCP.
func wsDial(ctx context.Context, addr, host, path string, transport net.Conn) (net.Conn, error) {
	if transport == nil {
		dialer := &net.Dialer{Timeout: 10 * time.Second}
		var err error
		transport, err = dialer.DialContext(ctx, "tcp", addr)
		if err != nil {
			return nil, fmt.Errorf("sshchain: ws dial %s: %w", addr, err)
		}
	}
	if host == "" {
		host = wsHostFrom(addr)
	}
	if path == "" {
		path = "/tunnel"
	}

	// Handshake key (16 random bytes, base64).
	keyBytes := make([]byte, 16)
	if _, err := rand.Read(keyBytes); err != nil {
		transport.Close()
		return nil, err
	}
	key := base64.StdEncoding.EncodeToString(keyBytes)

	request := "GET " + path + " HTTP/1.1\r\n" +
		"Host: " + host + "\r\n" +
		"Upgrade: websocket\r\n" +
		"Connection: Upgrade\r\n" +
		"Sec-WebSocket-Key: " + key + "\r\n" +
		"Sec-WebSocket-Version: 13\r\n\r\n"
	if _, err := transport.Write([]byte(request)); err != nil {
		transport.Close()
		return nil, fmt.Errorf("sshchain: ws write: %w", err)
	}

	// Read the 101 response (headers only).
	reader := bufio.NewReader(transport)
	response, err := http.ReadResponse(reader, &http.Request{Method: "GET", URL: &url.URL{Path: path}})
	if err != nil {
		transport.Close()
		return nil, fmt.Errorf("sshchain: ws response: %w", err)
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusSwitchingProtocols {
		transport.Close()
		return nil, fmt.Errorf("sshchain: ws: unexpected status %d", response.StatusCode)
	}
	if !strings.EqualFold(response.Header.Get("Upgrade"), "websocket") {
		transport.Close()
		return nil, errors.New("sshchain: ws: missing Upgrade header")
	}
	// Verify the accept key: SHA1(key + GUID), base64.
	expect := wsAcceptKey(key)
	if response.Header.Get("Sec-WebSocket-Accept") != expect {
		transport.Close()
		return nil, errors.New("sshchain: ws: bad Sec-WebSocket-Accept")
	}

	// Bytes already buffered by the reader must survive.
	var buffered []byte
	if reader.Buffered() > 0 {
		buffered, _ = reader.Peek(reader.Buffered())
	}
	return &wsConn{Conn: transport, rbuf: buffered, writeMu: make(chan struct{}, 1)}, nil
}

// wsConn wraps the raw stream with RFC 6455 framing.
type wsConn struct {
	net.Conn
	rbuf    []byte // bytes read before the handshake completed
	writeMu chan struct{}
}

func (c *wsConn) Read(p []byte) (int, error) {
	for len(c.rbuf) == 0 {
		payload, opcode, err := c.readFrame()
		if err != nil {
			return 0, err
		}
		switch opcode {
		case opBinary, opText, opContinuation:
			c.rbuf = payload
		case opPing:
			// Control frames must be answerable even mid-stream.
			if err := c.writeFrame(opPong, payload); err != nil {
				return 0, err
			}
		case opPong:
			// Nothing to do.
		case opClose:
			return 0, io.EOF
		default:
			return 0, fmt.Errorf("sshchain: ws: unexpected opcode %d", opcode)
		}
	}
	n := copy(p, c.rbuf)
	c.rbuf = c.rbuf[n:]
	return n, nil
}

func (c *wsConn) Write(p []byte) (int, error) {
	// Chunk to keep frame sizes reasonable, mirroring common bridges.
	const max = 16 * 1024
	written := 0
	for len(p) > 0 {
		chunk := p
		if len(chunk) > max {
			chunk = chunk[:max]
		}
		if err := c.writeFrame(opBinary, chunk); err != nil {
			return written, err
		}
		written += len(chunk)
		p = p[len(chunk):]
	}
	return written, nil
}

// RFC 6455 opcodes.
const (
	opContinuation = 0x0
	opText         = 0x1
	opBinary       = 0x2
	opClose        = 0x8
	opPing         = 0x9
	opPong         = 0xA
)

// writeFrame writes one masked binary/text frame.
func (c *wsConn) writeFrame(opcode byte, payload []byte) error {
	c.writeMu <- struct{}{}
	defer func() { <-c.writeMu }()

	header := make([]byte, 0, 14)
	header = append(header, 0x80|opcode) // FIN + opcode
	length := len(payload)
	switch {
	case length < 126:
		header = append(header, byte(0x80|length)) // MASK bit set
	case length <= 0xFFFF:
		header = append(header, 0x80|126)
		var ext [2]byte
		binary.BigEndian.PutUint16(ext[:], uint16(length))
		header = append(header, ext[:]...)
	default:
		header = append(header, 0x80|127)
		var ext [8]byte
		binary.BigEndian.PutUint64(ext[:], uint64(length))
		header = append(header, ext[:]...)
	}
	mask := make([]byte, 4)
	if _, err := rand.Read(mask); err != nil {
		return err
	}
	header = append(header, mask...)
	if _, err := c.Conn.Write(header); err != nil {
		return err
	}
	masked := make([]byte, length)
	for i := 0; i < length; i++ {
		masked[i] = payload[i] ^ mask[i%4]
	}
	if length > 0 {
		if _, err := c.Conn.Write(masked); err != nil {
			return err
		}
	}
	return nil
}

// readFrame reads one frame and returns (payload, opcode, error). Control
// frames are small (<=125 bytes) by spec; data frames stream in full.
func (c *wsConn) readFrame() ([]byte, byte, error) {
	var head [2]byte
	if _, err := io.ReadFull(c.Conn, head[:]); err != nil {
		return nil, 0, err
	}
	fin := head[0]&0x80 != 0
	opcode := head[0] & 0x0F
	masked := head[1]&0x80 != 0
	length := int(head[1] & 0x7F)
	switch length {
	case 126:
		var ext [2]byte
		if _, err := io.ReadFull(c.Conn, ext[:]); err != nil {
			return nil, 0, err
		}
		length = int(binary.BigEndian.Uint16(ext[:]))
	case 127:
		var ext [8]byte
		if _, err := io.ReadFull(c.Conn, ext[:]); err != nil {
			return nil, 0, err
		}
		length = int(binary.BigEndian.Uint64(ext[:]))
	}
	var mask [4]byte
	if masked {
		if _, err := io.ReadFull(c.Conn, mask[:]); err != nil {
			return nil, 0, err
		}
	}
	if !fin {
		return nil, 0, errors.New("sshchain: ws: fragmentation not supported")
	}
	payload := make([]byte, length)
	if _, err := io.ReadFull(c.Conn, payload); err != nil {
		return nil, 0, err
	}
	if masked {
		for i := 0; i < length; i++ {
			payload[i] ^= mask[i%4]
		}
	}
	return payload, opcode, nil
}

func wsAcceptKey(key string) string {
	const guid = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
	sum := sha1.Sum([]byte(key + guid))
	return base64.StdEncoding.EncodeToString(sum[:])
}

func wsHostFrom(addr string) string {
	if host, _, err := net.SplitHostPort(addr); err == nil {
		return host
	}
	return addr
}

// httpConnectDial tunnels through an HTTP CONNECT proxy (custom Host header
// optional). Used by Config.WrapHTTP.
func httpConnectDial(ctx context.Context, proxyAddr, target, hostHeader string) (net.Conn, error) {
	dialer := &net.Dialer{Timeout: 10 * time.Second}
	conn, err := dialer.DialContext(ctx, "tcp", proxyAddr)
	if err != nil {
		return nil, fmt.Errorf("sshchain: proxy dial %s: %w", proxyAddr, err)
	}
	host := hostHeader
	if host == "" {
		host = target
	}
	request := "CONNECT " + target + " HTTP/1.1\r\n" +
		"Host: " + host + "\r\n" +
		"Proxy-Connection: keep-alive\r\n\r\n"
	if _, err := conn.Write([]byte(request)); err != nil {
		conn.Close()
		return nil, fmt.Errorf("sshchain: connect write: %w", err)
	}
	reader := bufio.NewReader(conn)
	response, err := http.ReadResponse(reader, &http.Request{Method: http.MethodConnect})
	if err != nil {
		conn.Close()
		return nil, fmt.Errorf("sshchain: connect response: %w", err)
	}
	response.Body.Close()
	if response.StatusCode != http.StatusOK {
		conn.Close()
		return nil, fmt.Errorf("sshchain: connect: status %d", response.StatusCode)
	}
	if reader.Buffered() > 0 {
		buffered, _ := reader.Peek(reader.Buffered())
		return &prefixConn{Conn: conn, prefix: buffered}, nil
	}
	return conn, nil
}

// prefixConn replays bytes that were read ahead before handing off.
type prefixConn struct {
	net.Conn
	prefix []byte
}

func (c *prefixConn) Read(p []byte) (int, error) {
	if len(c.prefix) > 0 {
		n := copy(p, c.prefix)
		c.prefix = c.prefix[n:]
		return n, nil
	}
	return c.Conn.Read(p)
}
