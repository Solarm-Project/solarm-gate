Cleanup Existing packages
```bash
pkgrepo remove -p openindiana.org -s hipster/ '*'
```

Mirror only newest versions of packages into repo
```bash
cd /zdata/oirepo
pkgrecv -s http://pkg.openindiana.org/hipster --newest | tee pkg-list
cat pkg-list | split -l 100
for i in $(ls x*); do pkgrecv -s http://pkg.openindiana.org/hipster -d hipster/ $(cat $i); done
```

Check new repo (diff should be empty)
```bash
pkgrecv -s http://pkg.openindiana.org/hipster --newest > /tmp/old.packages  
pkgrecv -s hipster/ --newest > /tmp/new.packages  
diff -u /tmp/old.packages /tmp/new.packages  
```

Cleanup
```bash
pkgrepo -s hipster/ rebuild  
rm x* pkg-list  
```

Followup updates
```bash
pkgrecv -s http://pkg.openindiana.org/hipster -d hipster/
```