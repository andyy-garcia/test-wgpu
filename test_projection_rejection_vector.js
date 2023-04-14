// Derived (and to be executed) from https://www.khanacademy.org/computer-programming/vector-subtraction-ray-normalized/4917246077960192
    // Adapted from Dan Shiffman, natureofcode.com

// See NOTICE for license (for original work and this derived work, both available under MIT license).

function lineTo(fromX, fromY, toX, toY, colorX, colorY, colorZ) {
    pushMatrix();
    translate(fromX, fromY);
    stroke(colorX, colorY, colorZ);
    strokeWeight(3);
    line(0, 0, toX, toY);
    popMatrix();
}

function dot(a, b) {
    return (a.x * b.x) + (a.y * b.y);
}

function mulvec(vec, s) {
    return new PVector(vec.x * s, vec.y * s);
}

function proj(a, b) {
    return mulvec(b, dot(a, b) / dot(b, b));
}

function reject(a, b) {
    var c = new PVector(a.x, a.y);
    c.sub(proj(a, b));
    return c;
}

function ang(a, b) {
    var costheta = dot(a, b) / (a.mag() * b.mag());
    return acos(costheta);
}

mouseMoved = function() {
    background(255, 255, 255);
    
    var center = new PVector(width / 2, height / 2);

    var mouse  = new PVector(mouseX, mouseY);
    mouse.sub(center);
    
    lineTo(center.x, center.y, mouse.x, mouse.y, 255, 0, 0);
    
    var rando = new PVector(width / 1.1, height / 3.1);
    rando.normalize();
    rando.mult(50);
    lineTo(center.x, center.y, rando.x, rando.y, 0, 255, 0);
    
    fill(255, 0, 0);
    text(ang(mouse, rando), mouse.x+center.x, mouse.y+center.y);
    
    var p = proj(mouse, rando);
    lineTo(center.x, center.y, p.x, p.y, 0, 0, 255);
    
    var p2 = reject(mouse, rando);
    lineTo(center.x, center.y, p2.x, p2.y, 120, 120, 120);
};